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
        let mut task_ids = Vec::new();

        for url in request.urls.iter() {
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

        let output_path = PathBuf::from(&request.download_path);
        
        // Build yt-dlp command
        let mut cmd = TokioCommand::new("yt-dlp");
        
        // Set output template
        let output_template = output_path.join("%(title)s.%(ext)s");
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

                // Extract filename
                if line.contains("[download] Destination:") {
                    if let Some(file_path) = line.split("Destination:").nth(1) {
                        let file_path = file_path.trim().to_string();
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
