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
use uuid::Uuid;

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
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
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
            task_ids.push(task_id.clone());

            let manager = self.clone();
            let req = request.clone();
            let url = url.clone();
            let app_handle = app_handle.clone();

            tokio::spawn(async move {
                if let Err(e) = manager.download_single(&task_id, &url, &req, app_handle).await {
                    manager.update_task(&task_id, |task| {
                        task.status = TaskStatus::Failed;
                        task.error = Some(e.to_string());
                    });
                }
            });
        }

        Ok(task_ids)
    }

    async fn download_single(
        &self,
        task_id: &str,
        url: &str,
        request: &DownloadRequest,
        app_handle: tauri::AppHandle,
    ) -> Result<(), MediaForgeError> {
        self.update_task(task_id, |task| {
            task.status = TaskStatus::Downloading;
        });

        // Re-validate URL and sanitize path (defensive programming)
        validate_youtube_url(url)?;
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
        let task_id_clone = task_id_str.clone();
        let app_handle_clone = app_handle.clone();

        // Parse progress from stdout
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
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
        });

        let status = child.wait().await.map_err(|e| {
            MediaForgeError::YtDlpError(format!("Failed to wait for yt-dlp: {}", e))
        })?;

        if status.success() {
            self.update_task(&task_id_clone, |task| {
                task.status = TaskStatus::Completed;
                task.progress = 100.0;
            });
            
            // Send notification
            if let Some(task) = self.get_task(&task_id_clone) {
                notifications::send_download_complete_notification(&app_handle, &task.name);
            }
            
            let _ = app_handle.emit("task-update", self.get_task(&task_id_clone));
            Ok(())
        } else {
            Err(MediaForgeError::YtDlpError(
                "Download failed with non-zero exit code".to_string(),
            ))
        }
    }

    pub fn pause_task(&self, task_id: &str) -> Result<(), MediaForgeError> {
        self.update_task(task_id, |task| {
            task.status = TaskStatus::Paused;
        });
        Ok(())
    }

    pub fn cancel_task(&self, task_id: &str) -> Result<(), MediaForgeError> {
        self.update_task(task_id, |task| {
            task.status = TaskStatus::Cancelled;
        });
        Ok(())
    }
}

impl Clone for DownloadManager {
    fn clone(&self) -> Self {
        Self {
            tasks: Arc::clone(&self.tasks),
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
}
