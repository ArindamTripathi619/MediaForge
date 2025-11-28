use crate::types::SystemInfo;
use std::process::Command;

pub fn check_system_dependencies() -> SystemInfo {
    let has_ytdlp = check_command_exists("yt-dlp");
    let has_ffmpeg = check_command_exists("ffmpeg");
    
    let ytdlp_path = if has_ytdlp {
        get_command_path("yt-dlp")
    } else {
        None
    };
    
    let ffmpeg_path = if has_ffmpeg {
        get_command_path("ffmpeg")
    } else {
        None
    };
    
    SystemInfo {
        has_ytdlp,
        has_ffmpeg,
        ytdlp_path,
        ffmpeg_path,
    }
}

fn check_command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn get_command_path(command: &str) -> Option<String> {
    Command::new("which")
        .arg(command)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

pub fn install_ytdlp() -> Result<String, String> {
    // Try to install yt-dlp using pip
    let output = Command::new("pip3")
        .args(["install", "--user", "yt-dlp"])
        .output()
        .map_err(|e| format!("Failed to run pip3: {}", e))?;
    
    if output.status.success() {
        Ok("yt-dlp installed successfully".to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to install yt-dlp: {}", error))
    }
}
