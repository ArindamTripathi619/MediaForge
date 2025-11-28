use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum MediaForgeError {
    #[error("Download failed: {0}")]
    DownloadError(String),
    
    #[error("Conversion failed: {0}")]
    ConversionError(String),
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("File system error: {0}")]
    FileSystemError(String),
    
    #[error("FFmpeg error: {0}")]
    FFmpegError(String),
    
    #[error("yt-dlp error: {0}")]
    YtDlpError(String),
    
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    
    #[error("Invalid settings: {0}")]
    InvalidSettings(String),
    
    #[error("System dependency missing: {0}")]
    MissingDependency(String),
}

impl From<std::io::Error> for MediaForgeError {
    fn from(err: std::io::Error) -> Self {
        MediaForgeError::FileSystemError(err.to_string())
    }
}

impl From<anyhow::Error> for MediaForgeError {
    fn from(err: anyhow::Error) -> Self {
        MediaForgeError::DownloadError(err.to_string())
    }
}
