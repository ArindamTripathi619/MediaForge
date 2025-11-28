use crate::converter::ConversionManager;
use crate::downloader::DownloadManager;
use crate::system::*;
use crate::types::*;
use tauri::State;

pub struct AppState {
    pub download_manager: DownloadManager,
    pub conversion_manager: ConversionManager,
}

#[tauri::command]
pub async fn check_dependencies() -> Result<SystemInfo, String> {
    Ok(check_system_dependencies())
}

#[tauri::command]
pub async fn install_ytdlp_command() -> Result<String, String> {
    install_ytdlp()
}

#[tauri::command]
pub async fn start_download(
    request: DownloadRequest,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    state
        .download_manager
        .start_download(request, app_handle)
        .await
        .map_err(|e| e.to_string())
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
pub async fn start_conversion(
    request: ConvertRequest,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    state
        .conversion_manager
        .start_conversion(request, app_handle)
        .await
        .map_err(|e| e.to_string())
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
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn select_directory() -> Result<String, String> {
    // This will be handled by the frontend using @tauri-apps/plugin-dialog
    Err("Use dialog plugin from frontend".to_string())
}
