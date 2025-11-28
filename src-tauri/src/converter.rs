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

/// Handle for managing conversion task lifecycle with cancellation support
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

    async fn cancel(self) -> Result<(), tokio::task::JoinError> {
        self.cancellation_token.cancel();
        self.join_handle.await
    }

    fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }
}

/// Validates input file paths to ensure they exist and are not system files
fn validate_input_file(file_path: &PathBuf) -> Result<(), MediaForgeError> {
    // Check if file exists
    if !file_path.exists() {
        return Err(MediaForgeError::FileSystemError(
            format!("Input file does not exist: {}", file_path.display())
        ));
    }
    
    // Check if it's actually a file (not directory)
    if !file_path.is_file() {
        return Err(MediaForgeError::FileSystemError(
            format!("Path is not a file: {}", file_path.display())
        ));
    }
    
    // Prevent access to system files
    let path_str = file_path.to_string_lossy();
    if path_str.starts_with("/etc") 
        || path_str.starts_with("/sys") 
        || path_str.starts_with("/proc")
        || path_str.starts_with("/boot")
        || path_str.starts_with("/root")
        || path_str.contains("/.ssh/")
        || path_str.contains("/.gnupg/")
    {
        return Err(MediaForgeError::InvalidSettings(
            "Access to system files is not allowed".into()
        ));
    }
    
    // Validate file extension for security (prevent executable files)
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        let dangerous_extensions = [
            "sh", "bash", "zsh", "fish", "csh", // Shell scripts
            "py", "pl", "rb", "php", "js", // Scripts
            "exe", "com", "bat", "cmd", // Windows executables
            "so", "dylib", "dll", // Libraries
            "deb", "rpm", "pkg", // Package files
        ];
        
        if dangerous_extensions.contains(&ext_lower.as_str()) {
            return Err(MediaForgeError::InvalidSettings(
                format!("File type not allowed for conversion: .{}", ext_lower)
            ));
        }
    }
    
    Ok(())
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

/// Validates image format for ImageMagick security (prevent dangerous delegates)
fn validate_image_format(input_path: &PathBuf, output_format: &str) -> Result<(), MediaForgeError> {
    // Check input file extension
    if let Some(input_ext) = input_path.extension().and_then(|e| e.to_str()) {
        let input_ext_lower = input_ext.to_lowercase();
        
        // Allowed input formats (avoid potentially dangerous formats)
        let safe_input_formats = [
            "png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif",
            "ico", "psd", // Common safe formats
        ];
        
        if !safe_input_formats.contains(&input_ext_lower.as_str()) {
            return Err(MediaForgeError::InvalidSettings(
                format!("Input image format not supported: .{}", input_ext_lower)
            ));
        }
    }
    
    // Validate output format
    let output_lower = output_format.to_lowercase();
    let safe_output_formats = [
        "png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif", "ico"
    ];
    
    if !safe_output_formats.contains(&output_lower.as_str()) {
        return Err(MediaForgeError::InvalidSettings(
            format!("Output image format not supported: .{}", output_format)
        ));
    }
    
    Ok(())
}

pub struct ConversionManager {
    tasks: Arc<DashMap<String, TaskProgress>>,
    task_handles: Arc<DashMap<String, TaskHandle>>,
}

impl ConversionManager {
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

    pub async fn cancel_task(&self, task_id: &str) -> Result<(), MediaForgeError> {
        // Cancel the running task if it exists
        if let Some((_, task_handle)) = self.task_handles.remove(task_id) {
            log::info!("Cancelling conversion task: {}", task_id);
            
            // Update status first
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Cancelled;
                task.error = Some("Task cancelled by user".to_string());
            });
            
            // Actually cancel the running task
            if let Err(e) = task_handle.cancel().await {
                log::error!("Error cancelling conversion task {}: {:?}", task_id, e);
                return Err(MediaForgeError::TaskNotFound(format!("Failed to cancel task: {:?}", e)));
            }
            
            log::info!("Conversion task {} cancelled successfully", task_id);
        } else {
            // Task might not be running anymore, just update status
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Cancelled;
                task.error = Some("Task cancelled by user".to_string());
            });
            log::info!("Conversion task {} not running, marked as cancelled", task_id);
        }
        
        Ok(())
    }

    pub async fn start_conversion(
        &self,
        request: ConvertRequest,
        app_handle: tauri::AppHandle,
    ) -> Result<Vec<String>, MediaForgeError> {
        log::info!("Starting conversion for {} files", request.input_files.len());
        
        // Validate output path before processing any files
        let _sanitized_output_path = sanitize_path(&request.output_path)?;
        
        let mut task_ids = Vec::new();

        for input_file in request.input_files.iter() {
            // Validate each input file before creating task
            validate_input_file(input_file)?;
            
            // Additional validation for image files
            if request.conversion_type == ConversionType::Image {
                validate_image_format(input_file, &request.output_format)?;
            }
            
            let file_name = input_file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");
            log::info!("Creating conversion task for: {}", file_name);
            let task_id = self.create_task(format!("Converting {}", file_name));
            task_ids.push(task_id.clone());

            // Set task to Processing status BEFORE spawning to prevent race condition
            self.update_task(&task_id, |task| {
                task.status = TaskStatus::Processing;
            });

            let manager = self.clone();
            let req = request.clone();
            let input_file = input_file.clone();
            let app_handle = app_handle.clone();
            let app_handle_clone2 = app_handle.clone();
            let task_id_clone = task_id.clone();
            
            // Create cancellation token for this task
            let cancellation_token = CancellationToken::new();
            let cancellation_token_clone = cancellation_token.clone();

            let join_handle = tokio::spawn(async move {
                log::info!("Spawned conversion task: {}", task_id_clone);
                
                // Run the conversion with timeout and cancellation support
                let result = tokio::select! {
                    result = manager.convert_single_cancellable(&task_id_clone, &input_file, &req, app_handle.clone(), cancellation_token_clone.clone()) => {
                        result
                    }
                    _ = cancellation_token_clone.cancelled() => {
                        log::info!("Conversion task {} was cancelled", task_id_clone);
                        manager.update_task(&task_id_clone, |task| {
                            task.status = TaskStatus::Cancelled;
                            task.error = Some("Task was cancelled by user".to_string());
                        });
                        // Clean up task handle on cancellation
                        manager.task_handles.remove(&task_id_clone);
                        return;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(7200)) => { // 2 hour timeout for conversions
                        log::warn!("Conversion task {} timed out after 2 hours", task_id_clone);
                        manager.update_task(&task_id_clone, |task| {
                            task.status = TaskStatus::Failed;
                            task.error = Some("Conversion timed out after 2 hours".to_string());
                        });
                        // Clean up task handle on timeout
                        manager.task_handles.remove(&task_id_clone);
                        return;
                    }
                };
                
                if let Err(e) = result {
                    log::error!("Conversion failed for {}: {}", task_id_clone, e);
                    manager.update_task(&task_id_clone, |task| {
                        task.status = TaskStatus::Failed;
                        task.error = Some(e.to_string());
                    });
                    // Clean up task handle on error
                    manager.task_handles.remove(&task_id_clone);
                }
                
                // Emit final task update
                let _ = app_handle_clone2.emit("task-update", manager.get_task(&task_id_clone));
            });
            
            // Store the task handle for cancellation
            let task_handle = TaskHandle::new(join_handle, cancellation_token);
            self.task_handles.insert(task_id.clone(), task_handle);
        }

        Ok(task_ids)
    }

    async fn convert_single(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        app_handle: tauri::AppHandle,
    ) -> Result<(), MediaForgeError> {
        self.update_task(task_id, |task| {
            task.status = TaskStatus::Processing;
        });

        match request.conversion_type {
            ConversionType::Image => {
                self.convert_image(task_id, input_file, request, app_handle).await
            }
            ConversionType::Video => {
                self.convert_video(task_id, input_file, request, app_handle).await
            }
            ConversionType::Audio => {
                self.convert_audio(task_id, input_file, request, app_handle).await
            }
        }
    }

    async fn convert_single_cancellable(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        app_handle: tauri::AppHandle,
        cancellation_token: CancellationToken,
    ) -> Result<(), MediaForgeError> {
        // Task status is already set to Processing before spawn to prevent race condition
        
        match request.conversion_type {
            ConversionType::Image => {
                self.convert_image_cancellable(task_id, input_file, request, app_handle, cancellation_token).await
            }
            ConversionType::Video => {
                self.convert_video_cancellable(task_id, input_file, request, app_handle, cancellation_token).await
            }
            ConversionType::Audio => {
                self.convert_audio_cancellable(task_id, input_file, request, app_handle, cancellation_token).await
            }
        }
    }

    async fn convert_image(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        app_handle: tauri::AppHandle,
    ) -> Result<(), MediaForgeError> {
        // Re-validate inputs (defensive programming)
        validate_input_file(input_file)?;
        validate_image_format(input_file, &request.output_format)?;
        
        let file_stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MediaForgeError::InvalidSettings("Invalid input filename".to_string()))?;

        // Use sanitized output path
        let output_dir = sanitize_path(&request.output_path)?;
        let output_path = output_dir.join(format!("{}.{}", file_stem, request.output_format));

        // Use ImageMagick 7+ for image conversion (just 'magick', not 'magick convert')
        let mut cmd = TokioCommand::new("magick");
        cmd.arg(input_file);

        // Special handling for ICO format - must resize to valid icon sizes
        if request.output_format == "ico" {
            // ICO format has size limits, resize to 256x256 (max standard size)
            cmd.arg("-resize").arg("256x256");
            log::info!("ICO format detected, resizing to 256x256");
        }

        // Apply image settings
        if let Some(settings) = &request.image_settings {
            if let Some(quality) = settings.quality {
                cmd.arg("-quality").arg(quality.to_string());
                log::info!("Applying image quality: {}", quality);
            }

            if let Some(resize) = &settings.resize {
                // Skip resize if ICO format (already handled above)
                if request.output_format != "ico" {
                    log::info!("Applying image resize: {}", resize);
                    cmd.arg("-resize").arg(resize);
                }
            }
        }

        cmd.arg(&output_path);

        log::info!("Starting image conversion: {:?}", cmd);
        log::info!("Output path: {:?}", output_path);
        
        let output = cmd.output().await.map_err(|e| {
            log::error!("Failed to run ImageMagick: {}", e);
            MediaForgeError::ConversionError(format!("Failed to run ImageMagick: {}", e))
        })?;

        if output.status.success() {
            log::info!("Image conversion completed successfully: {:?}", output_path);
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Completed;
                task.progress = 100.0;
                task.file_path = Some(output_path.to_string_lossy().to_string());
            });
            
            // Send notification
            if let Some(task) = self.get_task(task_id) {
                notifications::send_conversion_complete_notification(&app_handle, &task.name);
            }
            
            let _ = app_handle.emit("task-update", self.get_task(task_id));
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            log::error!("ImageMagick conversion failed: {}", error);
            Err(MediaForgeError::ConversionError(format!(
                "ImageMagick failed: {}",
                error
            )))
        }
    }

    async fn convert_video(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        app_handle: tauri::AppHandle,
    ) -> Result<(), MediaForgeError> {
        // Re-validate inputs (defensive programming)
        validate_input_file(input_file)?;
        
        let file_stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MediaForgeError::InvalidSettings("Invalid input filename".to_string()))?;

        // Use sanitized output path
        let output_dir = sanitize_path(&request.output_path)?;
        let output_path = output_dir.join(format!("{}.{}", file_stem, request.output_format));

        log::info!("Starting video conversion from {:?} to {:?}", input_file, output_path);

        let mut cmd = TokioCommand::new("ffmpeg");
        cmd.arg("-i").arg(input_file);

        // Apply video settings
        if let Some(settings) = &request.video_settings {
            if let Some(resolution) = &settings.resolution {
                if resolution != "Keep Original" {
                    cmd.arg("-s").arg(resolution);
                }
            }

            if let Some(bitrate) = &settings.bitrate {
                if bitrate != "Keep Original" {
                    cmd.arg("-b:v").arg(bitrate);
                }
            }
        }

        // Progress monitoring
        cmd.arg("-progress").arg("pipe:1");
        cmd.arg("-y"); // Overwrite output files
        cmd.arg(&output_path);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        log::info!("FFmpeg command: {:?}", cmd);

        let mut child = cmd.spawn().map_err(|e| {
            log::error!("Failed to spawn FFmpeg: {}", e);
            MediaForgeError::FFmpegError(format!("Failed to spawn FFmpeg: {}", e))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            MediaForgeError::FFmpegError("Failed to capture stdout".to_string())
        })?;

        let stderr = child.stderr.take().ok_or_else(|| {
            MediaForgeError::FFmpegError("Failed to capture stderr".to_string())
        })?;

        let manager = self.clone();
        let task_id_clone = task_id.to_string();
        let app_handle_clone = app_handle.clone();

        // Parse FFmpeg progress
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(progress) = parse_ffmpeg_progress(&line) {
                    manager.update_task(&task_id_clone, |task| {
                        task.progress = progress;
                    });

                    let _ = app_handle_clone.emit("task-update", manager.get_task(&task_id_clone));
                }
            }
        });

        // Capture stderr for error logging
        let task_id_stderr = task_id.to_string();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut error_output = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                error_output.push_str(&line);
                error_output.push('\n');
            }

            if !error_output.is_empty() {
                log::error!("FFmpeg stderr for {}: {}", task_id_stderr, error_output);
            }
        });

        let status = child.wait().await.map_err(|e| {
            log::error!("Failed to wait for FFmpeg: {}", e);
            MediaForgeError::FFmpegError(format!("Failed to wait for FFmpeg: {}", e))
        })?;

        if status.success() {
            log::info!("Video conversion completed successfully: {:?}", output_path);
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Completed;
                task.progress = 100.0;
                task.file_path = Some(output_path.to_string_lossy().to_string());
            });
            
            // Send notification
            if let Some(task) = self.get_task(task_id) {
                notifications::send_conversion_complete_notification(&app_handle, &task.name);
            }
            
            let _ = app_handle.emit("task-update", self.get_task(task_id));
            Ok(())
        } else {
            log::error!("Video conversion failed with exit code: {:?}", status.code());
            Err(MediaForgeError::FFmpegError(
                format!("Conversion failed with exit code: {:?}", status.code()),
            ))
        }
    }

    async fn convert_video_cancellable(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        app_handle: tauri::AppHandle,
        cancellation_token: CancellationToken,
    ) -> Result<(), MediaForgeError> {
        // Re-validate inputs (defensive programming)
        validate_input_file(input_file)?;
        
        let file_stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MediaForgeError::InvalidSettings("Invalid input filename".to_string()))?;

        // Use sanitized output path
        let output_dir = sanitize_path(&request.output_path)?;
        let output_path = output_dir.join(format!("{}.{}", file_stem, request.output_format));

        // Validate disk space and permissions before starting
        let estimated_size = input_file.metadata()
            .map(|m| m.len() * 2) // Estimate 2x input size for conversion
            .unwrap_or(500 * 1024 * 1024); // Default 500MB
        crate::error::validation::validate_disk_space(&output_dir, Some(estimated_size)).await?;
        crate::error::validation::validate_write_permissions(&output_dir).await?;

        log::info!("Starting cancellable video conversion from {:?} to {:?}", input_file, output_path);
        
        // Use retry mechanism for conversion operations (filesystem errors mainly)
        let retry_config = crate::error::RetryConfig::for_filesystem();
        let conversion_result = crate::error::retry_async(retry_config, || {
            self.convert_video_attempt(task_id, input_file, request, &output_path, app_handle.clone(), cancellation_token.clone())
        }).await;
        
        // Cleanup on failure
        if let Err(ref error) = conversion_result {
            log::error!("Video conversion failed after retries for task {}: {}", task_id, error);
            let _ = crate::error::validation::cleanup_on_error(&output_path).await;
        }
        
        conversion_result
    }

    async fn convert_video_attempt(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        output_path: &PathBuf,
        app_handle: tauri::AppHandle,
        cancellation_token: CancellationToken,
    ) -> Result<(), MediaForgeError> {
        log::info!("Attempting video conversion: {:?} -> {:?}", input_file, output_path);

        let mut cmd = TokioCommand::new("ffmpeg");
        cmd.arg("-i").arg(input_file);

        // Apply video settings
        if let Some(settings) = &request.video_settings {
            if let Some(resolution) = &settings.resolution {
                if resolution != "Keep Original" {
                    cmd.arg("-s").arg(resolution);
                }
            }

            if let Some(bitrate) = &settings.bitrate {
                if bitrate != "Keep Original" {
                    cmd.arg("-b:v").arg(bitrate);
                }
            }
        }

        // Progress monitoring
        cmd.arg("-progress").arg("pipe:1");
        cmd.arg("-y"); // Overwrite output files
        cmd.arg(&output_path);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        log::info!("FFmpeg command: {:?}", cmd);

        let mut child = cmd.spawn().map_err(|e| {
            log::error!("Failed to spawn FFmpeg: {}", e);
            MediaForgeError::FFmpegError(format!("Failed to spawn FFmpeg: {}", e))
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            MediaForgeError::FFmpegError("Failed to capture stdout".to_string())
        })?;

        let _stderr = child.stderr.take().ok_or_else(|| {
            MediaForgeError::FFmpegError("Failed to capture stderr".to_string())
        })?;

        let manager = self.clone();
        let task_id_str = task_id.to_string();
        let task_id_clone = task_id_str.clone();
        let app_handle_clone = app_handle.clone();
        let cancellation_token_clone = cancellation_token.clone();

        // Parse FFmpeg progress with cancellation support
        let progress_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            loop {
                tokio::select! {
                    result = lines.next_line() => {
                        match result {
                            Ok(Some(line)) => {
                                if let Some(progress) = parse_ffmpeg_progress(&line) {
                                    manager.update_task(&task_id_clone, |task| {
                                        task.progress = progress;
                                    });

                                    let _ = app_handle_clone.emit("task-update", manager.get_task(&task_id_clone));
                                }
                            }
                            Ok(None) => break, // EOF
                            Err(_) => break,   // Error reading
                        }
                    }
                    _ = cancellation_token_clone.cancelled() => {
                        log::info!("Progress parsing cancelled for conversion task {}", task_id_str);
                        break;
                    }
                }
            }
        });

        // Wait for process completion or cancellation
        let status = tokio::select! {
            status = child.wait() => {
                status.map_err(|e| {
                    MediaForgeError::FFmpegError(format!("Failed to wait for FFmpeg: {}", e))
                })?
            }
            _ = cancellation_token.cancelled() => {
                log::info!("Killing FFmpeg process for cancelled conversion task {}", task_id);
                // Kill the child process
                if let Err(e) = child.kill().await {
                    log::error!("Failed to kill FFmpeg process: {}", e);
                }
                // Wait briefly for cleanup
                let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
                
                // Cancel progress parsing
                progress_handle.abort();
                
                return Err(MediaForgeError::FFmpegError("Conversion was cancelled".to_string()));
            }
        };

        // Cancel progress parsing since process completed
        progress_handle.abort();

        if status.success() {
            log::info!("Video conversion completed successfully: {:?}", output_path);
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Completed;
                task.progress = 100.0;
                task.file_path = Some(output_path.to_string_lossy().to_string());
            });
            
            // Clean up task handle since task completed
            self.task_handles.remove(task_id);
            
            // Send notification
            if let Some(task) = self.get_task(task_id) {
                notifications::send_conversion_complete_notification(&app_handle, &task.name);
            }
            
            let _ = app_handle.emit("task-update", self.get_task(task_id));
            Ok(())
        } else {
            // Clean up task handle on failure too
            self.task_handles.remove(task_id);
            
            // Enhanced error classification for FFmpeg
            let error_message = format!("Conversion failed with exit code: {:?}", status.code());
            let error = Self::classify_ffmpeg_error(&error_message, status.code());
            
            log::error!("FFmpeg failed for task {}: {} (retryable: {})", 
                       task_id, error, error.is_retryable());
            
            Err(error)
        }
    }

    async fn convert_audio(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        app_handle: tauri::AppHandle,
    ) -> Result<(), MediaForgeError> {
        // Re-validate inputs (defensive programming)
        validate_input_file(input_file)?;
        
        let file_stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MediaForgeError::InvalidSettings("Invalid input filename".to_string()))?;

        // Use sanitized output path
        let output_dir = sanitize_path(&request.output_path)?;
        let output_path = output_dir.join(format!("{}.{}", file_stem, request.output_format));

        log::info!("Starting audio conversion from {:?} to {:?}", input_file, output_path);

        let mut cmd = TokioCommand::new("ffmpeg");
        cmd.arg("-i").arg(input_file);

        // Apply audio settings
        if let Some(settings) = &request.audio_settings {
            if let Some(bitrate) = &settings.bitrate {
                cmd.arg("-b:a").arg(format!("{}k", bitrate));
            }

            if let Some(sample_rate) = &settings.sample_rate {
                cmd.arg("-ar").arg(sample_rate);
            }
        }

        cmd.arg("-vn"); // No video
        cmd.arg("-y");
        cmd.arg(&output_path);

        log::info!("FFmpeg audio command: {:?}", cmd);

        let output = cmd.output().await.map_err(|e| {
            log::error!("Failed to run FFmpeg for audio: {}", e);
            MediaForgeError::FFmpegError(format!("Failed to run FFmpeg: {}", e))
        })?;

        if output.status.success() {
            log::info!("Audio conversion completed successfully: {:?}", output_path);
            self.update_task(task_id, |task| {
                task.status = TaskStatus::Completed;
                task.progress = 100.0;
                task.file_path = Some(output_path.to_string_lossy().to_string());
            });
            
            // Send notification
            if let Some(task) = self.get_task(task_id) {
                notifications::send_conversion_complete_notification(&app_handle, &task.name);
            }
            
            let _ = app_handle.emit("task-update", self.get_task(task_id));
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            log::error!("Audio conversion failed: {}", error);
            Err(MediaForgeError::FFmpegError(format!("FFmpeg failed: {}", error)))
        }
    }
    
    /// Classifies FFmpeg errors to determine if they're retryable  
    pub fn classify_ffmpeg_error(message: &str, exit_code: Option<i32>) -> MediaForgeError {
        let msg_lower = message.to_lowercase();
        
        // Temporary filesystem errors (retryable)
        if msg_lower.contains("resource temporarily unavailable") ||
           msg_lower.contains("device busy") ||
           msg_lower.contains("interrupted system call") ||
           exit_code == Some(1)  // Generic failure that might be temporary
        {
            MediaForgeError::TemporaryError(message.to_string())
        }
        // Disk space errors  
        else if msg_lower.contains("no space left") ||
                msg_lower.contains("disk full") ||
                msg_lower.contains("write error")
        {
            MediaForgeError::DiskSpaceError(message.to_string())
        }
        // Permission errors
        else if msg_lower.contains("permission denied") ||
                msg_lower.contains("access denied") ||
                exit_code == Some(126)  // Permission denied
        {
            MediaForgeError::PermissionError(message.to_string())
        }
        // Invalid format/codec errors (not retryable)
        else if msg_lower.contains("invalid data") ||
                msg_lower.contains("unsupported") ||
                msg_lower.contains("no decoder") ||
                msg_lower.contains("invalid file format") ||
                exit_code == Some(255)  // FFmpeg generic error
        {
            MediaForgeError::FFmpegError(message.to_string())
        }
        // Default to retryable for other cases
        else {
            MediaForgeError::TemporaryError(message.to_string())
        }
    }

    async fn convert_image_cancellable(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        app_handle: tauri::AppHandle,
        _cancellation_token: CancellationToken,
    ) -> Result<(), MediaForgeError> {
        // For now, delegate to existing method
        // TODO: Add proper cancellation support for ImageMagick processes
        let result = self.convert_image(task_id, input_file, request, app_handle).await;
        
        // Clean up task handle on completion
        if result.is_ok() {
            self.task_handles.remove(task_id);
        } else {
            // Clean up task handle on failure too
            self.task_handles.remove(task_id);
        }
        
        result
    }

    async fn convert_audio_cancellable(
        &self,
        task_id: &str,
        input_file: &PathBuf,
        request: &ConvertRequest,
        app_handle: tauri::AppHandle,
        _cancellation_token: CancellationToken,
    ) -> Result<(), MediaForgeError> {
        // For now, delegate to existing method
        // TODO: Add proper cancellation support for FFmpeg audio processes
        let result = self.convert_audio(task_id, input_file, request, app_handle).await;
        
        // Clean up task handle on completion
        if result.is_ok() {
            self.task_handles.remove(task_id);
        } else {
            // Clean up task handle on failure too
            self.task_handles.remove(task_id);
        }
        
        result
    }
}

impl Clone for ConversionManager {
    fn clone(&self) -> Self {
        Self {
            tasks: Arc::clone(&self.tasks),
            task_handles: Arc::clone(&self.task_handles),
        }
    }
}

/// FFmpeg progress tracker to calculate actual percentage based on duration
struct FFmpegProgress {
    duration_ms: Option<u64>,
    current_ms: u64,
}

impl FFmpegProgress {
    fn new() -> Self {
        Self {
            duration_ms: None,
            current_ms: 0,
        }
    }
    
    fn parse_line(&mut self, line: &str) -> Option<f32> {
        // Parse duration from FFmpeg output (appears early in the output)
        // Format: "Duration: 00:01:23.45, start: 0.000000, bitrate: 1234 kb/s"
        if line.contains("Duration:") {
            let re = Regex::new(r"Duration: (\d+):(\d+):(\d+)\.(\d+)").unwrap();
            if let Some(caps) = re.captures(line) {
                let hours: u64 = caps[1].parse().unwrap_or(0);
                let mins: u64 = caps[2].parse().unwrap_or(0);
                let secs: u64 = caps[3].parse().unwrap_or(0);
                let ms: u64 = caps[4].parse().unwrap_or(0) * 10; // Convert to ms
                
                self.duration_ms = Some((hours * 3600 + mins * 60 + secs) * 1000 + ms);
                log::info!("FFmpeg detected duration: {} ms", self.duration_ms.unwrap());
            }
        }
        
        // Parse current time from progress output
        // Format: "out_time_ms=12345678"
        if line.starts_with("out_time_ms=") {
            if let Some(ms_str) = line.strip_prefix("out_time_ms=") {
                self.current_ms = ms_str.trim().parse().unwrap_or(0);
                
                // Calculate percentage if we have duration
                if let Some(total) = self.duration_ms {
                    if total > 0 {
                        let progress = (self.current_ms as f32 / total as f32 * 100.0).min(100.0);
                        return Some(progress);
                    }
                }
            }
        }
        
        None
    }
}

fn parse_ffmpeg_progress(line: &str) -> Option<f32> {
    // This is a simplified version - in practice, you'd want to maintain 
    // a progress tracker per task. For now, we'll do basic parsing.
    if line.starts_with("out_time_ms=") {
        // Try to extract meaningful progress
        if let Some(ms_str) = line.strip_prefix("out_time_ms=") {
            if let Ok(current_ms) = ms_str.trim().parse::<u64>() {
                // Rough heuristic: assume most conversions are < 10 minutes
                // This is not accurate but better than always 50%
                let estimated_total_ms = 600_000; // 10 minutes
                let progress = (current_ms as f32 / estimated_total_ms as f32 * 100.0).min(100.0);
                if progress > 0.0 && progress <= 100.0 {
                    return Some(progress);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod error_recovery_tests {
    use super::*;
    use crate::error::{MediaForgeError, RetryConfig, retry_async};
    
    #[test]
    fn test_error_classification() {
        // Test retryable errors
        let network_error = MediaForgeError::NetworkError("Connection timeout".to_string());
        assert!(network_error.is_retryable());
        
        let temp_error = MediaForgeError::TemporaryError("Resource unavailable".to_string());
        assert!(temp_error.is_retryable());
        
        // Test non-retryable errors
        let invalid_url = MediaForgeError::InvalidUrl("Not a valid YouTube URL".to_string());
        assert!(!invalid_url.is_retryable());
        
        let invalid_settings = MediaForgeError::InvalidSettings("Bad format".to_string());
        assert!(!invalid_settings.is_retryable());
    }
    
    #[test]
    fn test_ytdlp_error_classification() {
        use crate::downloader::DownloadManager;
        
        // Network errors (retryable)
        let network_err = DownloadManager::classify_ytdlp_error("network timeout occurred", Some(1));
        assert!(network_err.is_retryable());
        
        // Permission errors (not retryable) 
        let private_err = DownloadManager::classify_ytdlp_error("private video not available", Some(2));
        assert!(!private_err.is_retryable());
        
        // Disk space errors
        let disk_err = DownloadManager::classify_ytdlp_error("no space left on device", None);
        assert!(!disk_err.is_retryable()); // Disk space errors shouldn't be retried immediately
    }
    
    #[test]  
    fn test_ffmpeg_error_classification() {
        // Temporary errors (retryable)
        let temp_err = ConversionManager::classify_ffmpeg_error("resource temporarily unavailable", Some(1));
        assert!(temp_err.is_retryable());
        
        // Permission errors (not retryable)
        let perm_err = ConversionManager::classify_ffmpeg_error("permission denied", Some(126));
        assert!(!perm_err.is_retryable());
        
        // Invalid format errors (not retryable)
        let format_err = ConversionManager::classify_ffmpeg_error("unsupported codec", Some(255));
        assert!(!format_err.is_retryable());
    }
    
    #[test]
    fn test_retry_config() {
        let config = RetryConfig::for_network();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.base_delay, 3);
        
        // Test exponential backoff
        assert_eq!(config.calculate_delay(1), 3);  // Base delay for first retry
        assert_eq!(config.calculate_delay(2), 6);  // 2x base delay 
        assert_eq!(config.calculate_delay(3), 12); // 4x base delay
        assert!(config.calculate_delay(10) <= config.max_delay); // Capped at max
    }
    
    #[tokio::test]
    async fn test_retry_mechanism() {
        let mut call_count = 0;
        let config = RetryConfig {
            max_attempts: 3,
            base_delay: 0, // No delay for testing
            max_delay: 0,
            exponential_backoff: false,
        };
        
        // Test successful retry
        let result = retry_async(config.clone(), || {
            call_count += 1;
            async move {
                if call_count < 3 {
                    Err(MediaForgeError::NetworkError("Temporary failure".to_string()))
                } else {
                    Ok("Success".to_string())
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(call_count, 3);
        
        // Test non-retryable error  
        let result2: Result<String, MediaForgeError> = retry_async(config, || async {
            Err(MediaForgeError::InvalidUrl("Bad URL".to_string()))
        }).await;
        
        assert!(result2.is_err());
        // Should fail immediately without retries for non-retryable errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn test_conversion_task_handle_creation() {
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
    fn test_conversion_manager_task_handles() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = ConversionManager::new();
            
            // Create a task
            let task_id = manager.create_task("Test Conversion".to_string());
            
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
    fn test_child_process_cleanup() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = ConversionManager::new();
            
            // Create a task
            let task_id = manager.create_task("Test Process Cleanup".to_string());
            
            // Set task to Processing status (simulating process lifecycle)
            manager.update_task(&task_id, |task| {
                task.status = TaskStatus::Processing;
            });
            
            // Verify status was set before any async operations
            if let Some(task) = manager.get_task(&task_id) {
                assert_eq!(task.status, TaskStatus::Processing);
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

    #[test]
    fn test_cancel_conversion_task() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let manager = ConversionManager::new();
            
            // Create a task
            let task_id = manager.create_task("Test Cancellation".to_string());
            
            // Test cancelling non-running task
            let result = manager.cancel_task(&task_id).await;
            assert!(result.is_ok());
            
            // Verify task was marked as cancelled
            if let Some(task) = manager.get_task(&task_id) {
                assert_eq!(task.status, TaskStatus::Cancelled);
                assert!(task.error.is_some());
            }
        });
    }
}
