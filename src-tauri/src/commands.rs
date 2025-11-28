use crate::converter::ConversionManager;
use crate::downloader::DownloadManager;
use crate::system::*;
use crate::types::*;
use tauri::State;
use tracing::{info, error, instrument};
use uuid::Uuid;

pub struct AppState {
    pub download_manager: DownloadManager,
    pub conversion_manager: ConversionManager,
}

#[tauri::command]
#[instrument]
pub async fn check_dependencies() -> Result<SystemInfo, String> {
    let correlation_id = Uuid::new_v4().to_string();
    info!(
        correlation_id = correlation_id,
        command = "check_dependencies",
        "Checking system dependencies"
    );
    
    let result = check_system_dependencies();
    info!(
        correlation_id = correlation_id,
        has_ytdlp = result.has_ytdlp,
        has_ffmpeg = result.has_ffmpeg,
        ytdlp_path = ?result.ytdlp_path,
        ffmpeg_path = ?result.ffmpeg_path,
        "System dependency check completed"
    );
    
    Ok(result)
}

#[tauri::command]
#[instrument]
pub async fn install_ytdlp_command() -> Result<String, String> {
    let correlation_id = Uuid::new_v4().to_string();
    info!(
        correlation_id = correlation_id,
        command = "install_ytdlp_command",
        "Installing yt-dlp dependency"
    );
    
    match install_ytdlp() {
        Ok(result) => {
            info!(
                correlation_id = correlation_id,
                "yt-dlp installation completed successfully"
            );
            Ok(result)
        }
        Err(e) => {
            error!(
                correlation_id = correlation_id,
                error = %e,
                "yt-dlp installation failed"
            );
            Err(e)
        }
    }
}

#[tauri::command]
#[instrument(skip(state, app_handle))]
pub async fn start_download(
    request: DownloadRequest,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let correlation_id = Uuid::new_v4().to_string();
    info!(
        correlation_id = correlation_id,
        command = "start_download",
        url_count = request.urls.len(),
        format = ?request.format,
        quality = ?request.quality,
        "Starting download request"
    );
    
    match state
        .download_manager
        .start_download(request, app_handle)
        .await
    {
        Ok(task_ids) => {
            info!(
                correlation_id = correlation_id,
                task_count = task_ids.len(),
                task_ids = ?task_ids,
                "Download tasks created successfully"
            );
            Ok(task_ids)
        }
        Err(e) => {
            error!(
                correlation_id = correlation_id,
                error = %e,
                "Failed to start download"
            );
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_download_tasks(state: State<'_, AppState>) -> Result<Vec<TaskProgress>, String> {
    Ok(state.download_manager.get_all_tasks())
}

#[tauri::command]
pub async fn get_task_progress(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Option<TaskProgress>, String> {
    Ok(state.download_manager.get_task(&task_id))
}

#[tauri::command]
pub async fn pause_download(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .download_manager
        .pause_task(&task_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_download(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .download_manager
        .cancel_task(&task_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.download_manager.remove_task(&task_id);
    state.conversion_manager.remove_task(&task_id);
    Ok(())
}

#[tauri::command]
#[instrument(skip(state, app_handle))]
pub async fn start_conversion(
    request: ConvertRequest,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let correlation_id = Uuid::new_v4().to_string();
    info!(
        correlation_id = correlation_id,
        command = "start_conversion",
        input_file_count = request.input_files.len(),
        conversion_type = ?request.conversion_type,
        output_format = ?request.output_format,
        "Starting conversion request"
    );
    
    match state
        .conversion_manager
        .start_conversion(request, app_handle)
        .await
    {
        Ok(task_ids) => {
            info!(
                correlation_id = correlation_id,
                task_count = task_ids.len(),
                task_ids = ?task_ids,
                "Conversion tasks created successfully"
            );
            Ok(task_ids)
        }
        Err(e) => {
            error!(
                correlation_id = correlation_id,
                error = %e,
                "Failed to start conversion"
            );
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_conversion_tasks(state: State<'_, AppState>) -> Result<Vec<TaskProgress>, String> {
    Ok(state.conversion_manager.get_all_tasks())
}

#[tauri::command]
pub async fn cancel_conversion(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .conversion_manager
        .cancel_task(&task_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn select_directory() -> Result<String, String> {
    // This will be handled by the frontend using @tauri-apps/plugin-dialog
    Err("Use dialog plugin from frontend".to_string())
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    use std::process::Command;
    
    // Extract directory path if a file path is provided
    let folder_path = std::path::Path::new(&path);
    let folder = if folder_path.is_file() {
        folder_path.parent().unwrap_or(folder_path)
    } else {
        folder_path
    };
    
    // Open folder based on OS
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(folder)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}
