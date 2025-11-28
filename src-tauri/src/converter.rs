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
}

impl ConversionManager {
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

            let manager = self.clone();
            let req = request.clone();
            let input_file = input_file.clone();
            let app_handle = app_handle.clone();

            tokio::spawn(async move {
                log::info!("Spawned conversion task: {}", task_id);
                if let Err(e) = manager
                    .convert_single(&task_id, &input_file, &req, app_handle.clone())
                    .await
                {
                    log::error!("Conversion failed for {}: {}", task_id, e);
                    manager.update_task(&task_id, |task| {
                        task.status = TaskStatus::Failed;
                        task.error = Some(e.to_string());
                    });
                    // Emit the failed task update
                    let _ = app_handle.emit("task-update", manager.get_task(&task_id));
                }
            });
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

    pub fn cancel_task(&self, task_id: &str) -> Result<(), MediaForgeError> {
        self.update_task(task_id, |task| {
            task.status = TaskStatus::Cancelled;
        });
        Ok(())
    }
}

impl Clone for ConversionManager {
    fn clone(&self) -> Self {
        Self {
            tasks: Arc::clone(&self.tasks),
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
