use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub fn send_download_complete_notification(app: &AppHandle, filename: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Download Complete")
        .body(format!("Successfully downloaded: {}", filename))
        .icon("icons/icon.png")
        .show();
}

#[allow(dead_code)]
pub fn send_download_failed_notification(app: &AppHandle, filename: &str, error: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Download Failed")
        .body(format!("Failed to download {}: {}", filename, error))
        .icon("icons/icon.png")
        .show();
}

pub fn send_conversion_complete_notification(app: &AppHandle, filename: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Conversion Complete")
        .body(format!("Successfully converted: {}", filename))
        .icon("icons/icon.png")
        .show();
}

#[allow(dead_code)]
pub fn send_conversion_failed_notification(app: &AppHandle, filename: &str, error: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Conversion Failed")
        .body(format!("Failed to convert {}: {}", filename, error))
        .icon("icons/icon.png")
        .show();
}

#[allow(dead_code)]
pub fn send_playlist_complete_notification(app: &AppHandle, count: usize) {
    let _ = app
        .notification()
        .builder()
        .title("Playlist Download Complete")
        .body(format!("Successfully downloaded {} items", count))
        .icon("icons/icon.png")
        .show();
}
