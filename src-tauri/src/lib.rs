mod commands;
mod converter;
mod downloader;
mod error;
mod notifications;
mod system;
mod types;

use commands::*;
use converter::ConversionManager;
use downloader::DownloadManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        download_manager: DownloadManager::new(),
        conversion_manager: ConversionManager::new(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            check_dependencies,
            install_ytdlp_command,
            start_download,
            get_download_tasks,
            get_task_progress,
            pause_download,
            cancel_download,
            remove_task,
            start_conversion,
            get_conversion_tasks,
            cancel_conversion,
            open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
