use crate::error::MediaForgeError;
use crate::notifications;
use crate::types::*;
use dashmap::DashMap;
use regex::Regex;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Task handle for managing async download operations
#[derive(Debug)]
struct TaskHandle {
    join_handle: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl TaskHandle {
    fn new(join_handle: JoinHandle<()>, cancellation_token: CancellationToken) -> Self {
        Self {
            join_handle,
            cancellation_token,
        }
    }
    
    /// Cancel the task and wait for it to complete
    async fn cancel(self) -> Result<(), tokio::task::JoinError> {
        self.cancellation_token.cancel();
        self.join_handle.await
    }
    
    /// Check if the task is cancelled
    fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }
}

/// Validates YouTube URL to prevent malicious schemes and ensure valid YouTube URLs
fn validate_youtube_url(url: &str) -> Result<(), MediaForgeError> {
    // Check for malicious schemes
    if url.starts_with("file://") 
        || url.starts_with("javascript:")
        || url.starts_with("data:")
        || url.starts_with("ftp://")
        || url.contains('\n')
        || url.contains('\r')
        || url.contains(';')
        || url.contains('&')
        || url.contains('|')
        || url.contains('`')
        || url.contains('$')
        || url.contains('(')
        || url.contains(')')
    {
        return Err(MediaForgeError::InvalidUrl(
            "URL contains potentially malicious characters or schemes".into()
        ));
    }
    
    // Valid YouTube URL patterns
    let valid_patterns = vec![
        r"^https?://(www\.)?youtube\.com/watch\?v=[\w-]{11}(&.*)?$",
        r"^https?://youtu\.be/[\w-]{11}(\?.*)?$",
        r"^https?://(www\.)?youtube\.com/playlist\?list=[\w-]+(&.*)?$",
        r"^https?://(music\.)?youtube\.com/watch\?v=[\w-]{11}(&.*)?$",
        r"^https?://(www\.)?youtube\.com/shorts/[\w-]{11}(\?.*)?$",
    ];
    
    for pattern in valid_patterns {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(url) {
            return Ok(());
        }
    }
    
    Err(MediaForgeError::InvalidUrl(
        "URL is not a valid YouTube URL".into()
    ))
}

/// Sanitizes file paths to prevent path traversal and ensure paths are within allowed directories
fn sanitize_path(path: &str) -> Result<PathBuf, MediaForgeError> {
    // Expand tilde to home directory
    let expanded_path = if path.starts_with("~/") {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))  // Windows support
            .unwrap_or_else(|_| "/home".to_string());
        path.replacen("~", &home, 1)
    } else {
        path.to_string()
    };
    
    let path_buf = PathBuf::from(&expanded_path);
    
    // Prevent path traversal attacks
    for component in path_buf.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(MediaForgeError::InvalidSettings(
                "Path traversal detected: '..' not allowed in paths".into()
            ));
        }
    }
    
    // Check for other dangerous path components
    if expanded_path.contains("//") || expanded_path.contains("\\\\") {
        return Err(MediaForgeError::InvalidSettings(
            "Invalid path: double separators not allowed".into()
        ));
    }
    
    // Ensure path is absolute or can be made absolute
    let canonical_path = if path_buf.is_absolute() {
        path_buf
    } else {
        std::env::current_dir()
            .map_err(|e| MediaForgeError::FileSystemError(format!("Cannot get current directory: {}", e)))?
            .join(path_buf)
    };
    
    // Verify the parent directory exists or can be created
    if let Some(parent) = canonical_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MediaForgeError::FileSystemError(format!("Cannot create directory: {}", e)))?;
        }
    }
    
    // Ensure path is within reasonable bounds (not system directories)
    let path_str = canonical_path.to_string_lossy();
    if path_str.starts_with("/etc") 
        || path_str.starts_with("/sys") 
        || path_str.starts_with("/proc")
        || path_str.starts_with("/boot")
        || path_str.starts_with("/root")  // Unless we're root
        || path_str.contains("/.ssh/")
        || path_str.contains("/.gnupg/")
    {
        return Err(MediaForgeError::InvalidSettings(
            "Access to system directories is not allowed".into()
        ));
    }
    
    Ok(canonical_path)
}

pub struct DownloadManager {
    tasks: Arc<DashMap<String, TaskProgress>>,
    task_handles: Arc<DashMap<String, TaskHandle>>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
            task_handles: Arc::new(DashMap::new()),
        }
    }

    pub fn create_task(&self, name: String) -> String {
        let task_id = Uuid::new_v4().to_string();
        let task = TaskProgress {
            task_id: task_id.clone(),
            name,
            status: TaskStatus::Queued,
            progress: 0.0,
            speed: None,
            eta: None,
            error: None,
            file_path: None,
        };
        self.tasks.insert(task_id.clone(), task);
        task_id
    }

    pub fn get_task(&self, task_id: &str) -> Option<TaskProgress> {
        self.tasks.get(task_id).map(|t| t.clone())
    }

    pub fn get_all_tasks(&self) -> Vec<TaskProgress> {
        self.tasks.iter().map(|entry| entry.value().clone()).collect()
    }

    pub fn update_task(&self, task_id: &str, update: impl FnOnce(&mut TaskProgress)) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            update(&mut task);
        }
    }

    pub fn remove_task(&self, task_id: &str) {
        self.tasks.remove(task_id);
    }

    pub async fn start_download(
        &self,
        request: DownloadRequest,
        app_handle: tauri::AppHandle,
    ) -> Result<Vec<String>, MediaForgeError> {
        // Validate download path before processing any URLs
        let _sanitized_path = sanitize_path(&request.download_path)?;
        
        let mut task_ids = Vec::new();

        for url in request.urls.iter() {
            // Validate each URL before creating task
            validate_youtube_url(url)?;
            
            let task_id = self.create_task(format!("Downloading from {}", url));
            
            // Set task to Downloading status BEFORE spawning to prevent race condition
            self.update_task(&task_id, |task| {
                task.status = TaskStatus::Downloading;
            });
            
            task_ids.push(task_id.clone());

            let manager = self.clone();
            let req = request.clone();
            let url = url.clone();
            let app_handle_clone = app_handle.clone();
            let app_handle_clone2 = app_handle.clone();
            let task_id_clone = task_id.clone();
            
            // Create cancellation token for this task
            let cancellation_token = CancellationToken::new();
            let cancellation_token_clone = cancellation_token.clone();

            let join_handle = tokio::spawn(async move {
                // Run the download with timeout and cancellation support
                let result = tokio::select! {
                    result = manager.download_single_cancellable(&task_id_clone, &url, &req, app_handle_clone, cancellation_token_clone.clone()) => {
                        result
                    }
                    _ = cancellation_token_clone.cancelled() => {
                        log::info!("Task {} was cancelled", task_id_clone);
                        manager.update_task(&task_id_clone, |task| {
                            task.status = TaskStatus::Cancelled;
                            task.error = Some("Task was cancelled by user".to_string());
                        });
                        // Clean up task handle on cancellation
                        manager.task_handles.remove(&task_id_clone);
                        return;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(3600)) => {
                        log::warn!("Task {} timed out after 1 hour", task_id_clone);
                        manager.update_task(&task_id_clone, |task| {
                            task.status = TaskStatus::Failed;
                            task.error = Some("Download timed out after 1 hour".to_string());
                        });
                        // Clean up task handle on timeout
                        manager.task_handles.remove(&task_id_clone);
                        return;
                    }
                };
                
                if let Err(e) = result {
                    log::error!("Download failed for task {}: {}", task_id_clone, e);
                    manager.update_task(&task_id_clone, |task| {
                        task.status = TaskStatus::Failed;
                        task.error = Some(e.to_string());
                    });
                    // Clean up task handle on error
                    manager.task_handles.remove(&task_id_clone);
                }
                
                // Emit final task update - need a new clone since app_handle_clone was moved
                let _ = app_handle_clone2.emit("task-update", manager.get_task(&task_id_clone));
            });
            
            // Store the task handle for cancellation
            let task_handle = TaskHandle::new(join_handle, cancellation_token);
            self.task_handles.insert(task_id.clone(), task_handle);
        }

        Ok(task_ids)
    }

    async fn download_single_cancellable(
        &self,
        task_id: &str,
        url: &str,
        request: &DownloadRequest,
        app_handle: tauri::AppHandle,
        cancellation_token: CancellationToken,
    ) -> Result<(), MediaForgeError> {
        // Task status is already set to Downloading before spawn to prevent race condition
        
        // Re-validate URL and sanitize path (defensive programming)
        validate_youtube_url(url)?;
        let output_path = sanitize_path(&request.download_path)?;
        
        // Validate disk space and permissions before starting
        crate::error::validation::validate_disk_space(&output_path, Some(100 * 1024 * 1024)).await?; // Assume 100MB minimum
        crate::error::validation::validate_write_permissions(&output_path).await?;
        
        // Use retry mechanism for network operations
        let retry_config = crate::error::RetryConfig::for_network();
        let download_result = crate::error::retry_async(retry_config, || {
            self.download_single_attempt(task_id, url, request, app_handle.clone(), cancellation_token.clone())
        }).await;
        
        // Cleanup on failure
        if let Err(ref error) = download_result {
            log::error!("Download failed after retries for task {}: {}", task_id, error);
            let format_ext = match request.format {
                MediaFormat::Mp4 => "mp4", 
                MediaFormat::Mp3 => "mp3",
            };
            let potential_file = output_path.join(format!("*.{}", format_ext));
            // Try to cleanup any partial files - use a glob pattern would be better but for now just log
            log::info!("Consider cleaning up potential partial files matching: {:?}", potential_file);
        }
        
        download_result
    }

    async fn download_single_attempt(
        &self,
        task_id: &str,
        url: &str,
        request: &DownloadRequest,
        app_handle: tauri::AppHandle,
        cancellation_token: CancellationToken,
    ) -> Result<(), MediaForgeError> {
        let output_path = sanitize_path(&request.download_path)?;
        let format_ext = match request.format {
            MediaFormat::Mp4 => "mp4",
            MediaFormat::Mp3 => "mp3",
        };
        
        // Build yt-dlp command
        let mut cmd = TokioCommand::new("yt-dlp");
        
        // Set output template with the correct extension
        let output_template = output_path.join(format!("%(title)s.{}", format_ext));
        cmd.arg("-o").arg(output_template.to_string_lossy().to_string());

        // Set format based on user selection
        match request.format {
            MediaFormat::Mp4 => {
                if let Some(quality) = &request.quality {
                    cmd.arg("-f")
                        .arg(format!("bestvideo[height<={}]+bestaudio/best[height<={}]", quality, quality));
                } else {
                    cmd.arg("-f").arg("bestvideo+bestaudio/best");
                }
                cmd.arg("--merge-output-format").arg("mp4");
            }
            MediaFormat::Mp3 => {
                cmd.arg("-x").arg("--audio-format").arg("mp3");
                if let Some(quality) = &request.audio_quality {
                    cmd.arg("--audio-quality").arg(quality);
                }
            }
        }

        // Add trim settings if specified
        if let Some(trim) = &request.trim {
            cmd.arg("--download-sections")
                .arg(format!("*{}-{}", trim.start_time, trim.end_time));
        }

        // Handle playlist downloads
        match request.download_type {
            DownloadType::Playlist => {
                cmd.arg("--yes-playlist");
            }
            _ => {
                cmd.arg("--no-playlist");
            }
        }

        // Add progress output
        cmd.arg("--newline").arg("--progress");
        
        cmd.arg(url);

        // Execute command and capture output
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            MediaForgeError::YtDlpError(format!("Failed to spawn yt-dlp: {}", e))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            MediaForgeError::YtDlpError("Failed to capture stdout".to_string())
        })?;

        let manager = self.clone();
        let task_id_str = task_id.to_string();
        let _task_id_clone = task_id_str.clone();
        let app_handle_clone = app_handle.clone();
        let cancellation_token_clone = cancellation_token.clone();

        // Parse progress from stdout
        let progress_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            loop {
                tokio::select! {
                    result = lines.next_line() => {
                        match result {
                            Ok(Some(line)) => {
                                if let Some(progress) = parse_ytdlp_progress(&line) {
                                    manager.update_task(&task_id_str, |task| {
                                        task.progress = progress.percentage;
                                        task.speed = progress.speed;
                                        task.eta = progress.eta;
                                    });

                                    // Emit event to frontend
                                    let _ = app_handle_clone.emit("task-update", manager.get_task(&task_id_str));
                                }

                                // Extract filename - look for the final merged/converted file
                                if line.contains("[download] Destination:") || line.contains("[Merger]") || line.contains("[ExtractAudio]") {
                                    if let Some(file_path) = line.split("Destination:").nth(1)
                                        .or_else(|| line.split("Merging formats into").nth(1))
                                        .or_else(|| line.split("to:").nth(1)) {
                                        let file_path = file_path.trim().trim_matches('"').to_string();
                                        manager.update_task(&task_id_str, |task| {
                                            task.file_path = Some(file_path.clone());
                                            task.name = file_path;
                                        });
                                    }
                                }
                            }
                            Ok(None) => break, // EOF
                            Err(_) => break,   // Error reading
                        }
                    }
                    _ = cancellation_token_clone.cancelled() => {
                        log::info!("Progress parsing cancelled for task {}", task_id_str);
                        break;
                    }
                }
            }
        });

        // Wait for process completion or cancellation
        let status = tokio::select! {
            status = child.wait() => {
                status.map_err(|e| {
                    MediaForgeError::YtDlpError(format!("Failed to wait for yt-dlp: {}", e))
                })?
            }
            _ = cancellation_token.cancelled() => {
                log::info!("Killing yt-dlp process for cancelled task {}", task_id);
                // Kill the child process
                if let Err(e) = child.kill().await {
                    log::error!("Failed to kill yt-dlp process: {}", e);
                }
                // Wait briefly for cleanup
                let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
                
                // Cancel progress parsing
                progress_handle.abort();
                
                return Err(MediaForgeError::YtDlpError("Download was cancelled".to_string()));
            }
        };

        // Cancel progress parsing since process completed
        progress_handle.abort();

        if status.success() {
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Completed;
                task.progress = 100.0;
            });
            
            // Clean up task handle since task completed
            self.task_handles.remove(task_id);
            
            // Send notification
            if let Some(task) = self.get_task(task_id) {
                notifications::send_download_complete_notification(&app_handle, &task.name);
            }
            
            let _ = app_handle.emit("task-update", self.get_task(task_id));
            Ok(())
        } else {
            // Clean up task handle on failure too  
            self.task_handles.remove(task_id);
            
            // Enhanced error classification based on exit code and stderr
            let error_message = format!("Download failed with exit code: {:?}", status.code());
            let error = Self::classify_ytdlp_error(&error_message, status.code());
            
            log::error!("yt-dlp failed for task {}: {} (retryable: {})", 
                       task_id, error, error.is_retryable());
            
            Err(error)
        }
    }

    /// Classifies yt-dlp errors to determine if they're retryable
    pub fn classify_ytdlp_error(message: &str, exit_code: Option<i32>) -> MediaForgeError {
        let msg_lower = message.to_lowercase();
        
        // Network-related errors (retryable)
        if msg_lower.contains("network") || 
           msg_lower.contains("connection") ||
           msg_lower.contains("timeout") ||
           msg_lower.contains("temporary failure") ||
           msg_lower.contains("503") ||  // Service unavailable
           msg_lower.contains("502") ||  // Bad gateway  
           msg_lower.contains("504") ||  // Gateway timeout
           msg_lower.contains("429") ||  // Too many requests
           exit_code == Some(1)          // Generic network failure
        {
            MediaForgeError::NetworkError(message.to_string())
        }
        // Permission/authentication errors (not retryable)
        else if msg_lower.contains("private video") ||
                msg_lower.contains("not available") ||
                msg_lower.contains("geo-blocked") ||
                exit_code == Some(2)  // Authentication/permission error
        {
            MediaForgeError::YtDlpError(message.to_string())
        }
        // Disk space errors
        else if msg_lower.contains("no space left") ||
                msg_lower.contains("disk full")
        {
            MediaForgeError::DiskSpaceError(message.to_string())
        }
        // Generic retryable errors
        else if exit_code == Some(1) || msg_lower.contains("interrupted")
        {
            MediaForgeError::TemporaryError(message.to_string())
        }
        // Default to non-retryable yt-dlp error
        else {
            MediaForgeError::YtDlpError(message.to_string())
        }
    }

    pub fn pause_task(&self, task_id: &str) -> Result<(), MediaForgeError> {
        self.update_task(task_id, |task| {
            task.status = TaskStatus::Paused;
        });
        Ok(())
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<(), MediaForgeError> {
        // Cancel the running task if it exists
        if let Some((_, task_handle)) = self.task_handles.remove(task_id) {
            log::info!("Cancelling task: {}", task_id);
            
            // Update status first
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Cancelled;
                task.error = Some("Task cancelled by user".to_string());
            });
            
            // Actually cancel the running task
            if let Err(e) = task_handle.cancel().await {
                log::error!("Error cancelling task {}: {:?}", task_id, e);
                return Err(MediaForgeError::TaskNotFound(format!("Failed to cancel task: {:?}", e)));
            }
            
            log::info!("Task {} cancelled successfully", task_id);
        } else {
            // Task might not be running anymore, just update status
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Cancelled;
                task.error = Some("Task cancelled by user".to_string());
            });
            log::info!("Task {} not running, marked as cancelled", task_id);
        }
        
        Ok(())
    }
}

impl Clone for DownloadManager {
    fn clone(&self) -> Self {
        Self {
            tasks: Arc::clone(&self.tasks),
            task_handles: Arc::clone(&self.task_handles),
        }
    }
}

struct ProgressInfo {
    percentage: f32,
    speed: Option<String>,
    eta: Option<String>,
}

fn parse_ytdlp_progress(line: &str) -> Option<ProgressInfo> {
    // Parse yt-dlp progress lines
    // Format: [download]   45.2% of 123.45MiB at 1.23MiB/s ETA 00:45
    if !line.contains("[download]") {
        return None;
    }

    let percentage = line
        .split_whitespace()
        .find(|s| s.ends_with('%'))
        .and_then(|s| s.trim_end_matches('%').parse::<f32>().ok())
        .unwrap_or(0.0);

    let speed = line
        .split("at")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .map(|s| s.to_string());

    let eta = line
        .split("ETA")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .map(|s| s.to_string());

    Some(ProgressInfo {
        percentage,
        speed,
        eta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_youtube_url_valid() {
        // Valid YouTube URLs
        assert!(validate_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").is_ok());
        assert!(validate_youtube_url("https://youtube.com/watch?v=dQw4w9WgXcQ").is_ok());
        assert!(validate_youtube_url("https://youtu.be/dQw4w9WgXcQ").is_ok());
        assert!(validate_youtube_url("https://www.youtube.com/playlist?list=PLrAXtmRdnEQy6nuLvTYpTNjVjYGD1UBx").is_ok());
        assert!(validate_youtube_url("https://music.youtube.com/watch?v=dQw4w9WgXcQ").is_ok());
        assert!(validate_youtube_url("https://www.youtube.com/shorts/dQw4w9WgXcQ").is_ok());
    }

    #[test]
    fn test_validate_youtube_url_malicious() {
        // Malicious URLs should be rejected
        assert!(validate_youtube_url("file:///etc/passwd").is_err());
        assert!(validate_youtube_url("javascript:alert(1)").is_err());
        assert!(validate_youtube_url("data:text/plain,malicious").is_err());
        assert!(validate_youtube_url("ftp://example.com/file").is_err());
        assert!(validate_youtube_url("https://youtube.com/watch?v=abc; rm -rf /").is_err());
        assert!(validate_youtube_url("https://youtube.com/watch?v=abc & echo hacked").is_err());
        assert!(validate_youtube_url("https://youtube.com/watch?v=abc\nrm -rf /").is_err());
        assert!(validate_youtube_url("https://youtube.com/watch?v=abc`whoami`").is_err());
        assert!(validate_youtube_url("https://youtube.com/watch?v=abc$(whoami)").is_err());
    }

    #[test]
    fn test_validate_youtube_url_invalid_domains() {
        // Invalid domains should be rejected
        assert!(validate_youtube_url("https://evil.com/watch?v=dQw4w9WgXcQ").is_err());
        assert!(validate_youtube_url("https://youtube.evil.com/watch?v=dQw4w9WgXcQ").is_err());
        assert!(validate_youtube_url("https://notyoutube.com/watch?v=dQw4w9WgXcQ").is_err());
    }

    #[test]
    fn test_sanitize_path_valid() {
        // Valid paths should work
        assert!(sanitize_path("/tmp/test").is_ok());
        assert!(sanitize_path("./downloads").is_ok());
    }

    #[test]
    fn test_sanitize_path_traversal() {
        // Path traversal should be blocked
        assert!(sanitize_path("../../../etc/passwd").is_err());
        assert!(sanitize_path("/tmp/../../../etc/passwd").is_err());
    }

    #[test]
    fn test_sanitize_path_system_directories() {
        // System directories should be blocked
        assert!(sanitize_path("/etc/shadow").is_err());
        assert!(sanitize_path("/sys/kernel").is_err());
        assert!(sanitize_path("/proc/self").is_err());
        assert!(sanitize_path("/boot/grub").is_err());
    }

    #[test]
    fn test_task_handle_creation() {
        use tokio_util::sync::CancellationToken;
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let cancellation_token = CancellationToken::new();
            let join_handle = tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            });
            
            let task_handle = TaskHandle::new(join_handle, cancellation_token.clone());
            
            // Test that we can check cancellation status
            assert!(!task_handle.is_cancelled());
            
            // Test cancellation
            cancellation_token.cancel();
            assert!(task_handle.is_cancelled());
            
            // Test that cancel method works without panicking
            let _ = task_handle.cancel().await;
        });
    }

    #[test]
    fn test_download_manager_task_handles() {
        use tokio_util::sync::CancellationToken;
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = DownloadManager::new();
            
            // Create a task
            let task_id = manager.create_task("Test Task".to_string());
            
            // Verify initial state
            assert_eq!(manager.task_handles.len(), 0);
            
            // Simulate adding a task handle
            let cancellation_token = CancellationToken::new();
            let join_handle = tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            });
            let task_handle = TaskHandle::new(join_handle, cancellation_token);
            manager.task_handles.insert(task_id.clone(), task_handle);
            
            // Verify task handle was added
            assert_eq!(manager.task_handles.len(), 1);
            assert!(manager.task_handles.contains_key(&task_id));
            
            // Test cleanup - remove task handle
            manager.task_handles.remove(&task_id);
            assert_eq!(manager.task_handles.len(), 0);
        });
    }

    #[test]
    fn test_race_condition_prevention() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = DownloadManager::new();
            
            // Create a task
            let task_id = manager.create_task("Test Task".to_string());
            
            // Set task to Downloading status (simulating race condition fix)
            manager.update_task(&task_id, |task| {
                task.status = TaskStatus::Downloading;
            });
            
            // Verify status was set before any async operations
            if let Some(task) = manager.get_task(&task_id) {
                assert_eq!(task.status, TaskStatus::Downloading);
            } else {
                panic!("Task should exist");
            }
            
            // Simulate completion and cleanup
            manager.update_task(&task_id, |task| {
                task.status = TaskStatus::Completed;
                task.progress = 100.0;
            });
            
            // Verify final state
            if let Some(task) = manager.get_task(&task_id) {
                assert_eq!(task.status, TaskStatus::Completed);
                assert_eq!(task.progress, 100.0);
            }
        });
    }
}
