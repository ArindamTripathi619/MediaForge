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
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize structured logging with tracing
fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    let filter = if cfg!(debug_assertions) {
        // Development: More verbose logging with debug info
        EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("mediaforge=debug,info"))
            .unwrap()
    } else {
        // Production: Less verbose, structured logging
        EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("mediaforge=info,warn,error"))
            .unwrap()
    };

    let registry = tracing_subscriber::registry();

    if cfg!(debug_assertions) {
        // Development: Human-readable format with colors
        registry
            .with(filter)
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .pretty()
            )
            .init();
    } else {
        // Production: JSON format for log aggregation
        registry
            .with(filter)
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .json()
            )
            .init();
    }

    info!("Tracing initialized successfully");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        download_manager: DownloadManager::new(),
        conversion_manager: ConversionManager::new(),
    };

    // Initialize structured logging first
    init_tracing().expect("Failed to initialize tracing");
    
    info!("Starting MediaForge application");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|_app| {
            info!(
                app_name = "MediaForge",
                version = "1.0.0",
                debug_mode = cfg!(debug_assertions),
                "Application setup completed"
            );
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
