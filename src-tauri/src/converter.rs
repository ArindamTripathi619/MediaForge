use crate::error::MediaForgeError;
use crate::notifications;
use crate::types::*;
use dashmap::DashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use uuid::Uuid;

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
        let mut task_ids = Vec::new();

        for input_file in request.input_files.iter() {
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
        let file_stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MediaForgeError::InvalidSettings("Invalid input filename".to_string()))?;

        // Expand tilde in output path
        let output_path_str = if request.output_path.starts_with("~/") {
            request.output_path.replacen("~", &std::env::var("HOME").unwrap_or_else(|_| "/home".to_string()), 1)
        } else {
            request.output_path.clone()
        };

        let output_path = PathBuf::from(&output_path_str)
            .join(format!("{}.{}", file_stem, request.output_format));

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
        let file_stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MediaForgeError::InvalidSettings("Invalid input filename".to_string()))?;

        // Expand tilde in output path
        let output_path_str = if request.output_path.starts_with("~/") {
            request.output_path.replacen("~", &std::env::var("HOME").unwrap_or_else(|_| "/home".to_string()), 1)
        } else {
            request.output_path.clone()
        };

        let output_path = PathBuf::from(&output_path_str)
            .join(format!("{}.{}", file_stem, request.output_format));

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
        let file_stem = input_file
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MediaForgeError::InvalidSettings("Invalid input filename".to_string()))?;

        // Expand tilde in output path
        let output_path_str = if request.output_path.starts_with("~/") {
            request.output_path.replacen("~", &std::env::var("HOME").unwrap_or_else(|_| "/home".to_string()), 1)
        } else {
            request.output_path.clone()
        };

        let output_path = PathBuf::from(&output_path_str)
            .join(format!("{}.{}", file_stem, request.output_format));

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

fn parse_ffmpeg_progress(line: &str) -> Option<f32> {
    // FFmpeg progress format: out_time_ms=12345678
    if line.starts_with("out_time_ms=") {
        // This is simplified - in production, you'd calculate percentage based on duration
        // For now, we'll use a simple heuristic
        return Some(50.0); // Placeholder
    }
    None
}
