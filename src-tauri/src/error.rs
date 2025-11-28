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
    
    // Enhanced error recovery types
    #[error("Network error (retryable): {0}")]
    NetworkError(String),
    
    #[error("Insufficient disk space: {0}")]
    DiskSpaceError(String),
    
    #[error("Permission denied: {0}")]
    PermissionError(String),
    
    #[error("Temporary failure (retry recommended): {0}")]
    TemporaryError(String),
    
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
}

impl From<std::io::Error> for MediaForgeError {
    fn from(err: std::io::Error) -> Self {
        MediaForgeError::from_system_error(err)
    }
}

impl From<anyhow::Error> for MediaForgeError {
    fn from(err: anyhow::Error) -> Self {
        MediaForgeError::DownloadError(err.to_string())
    }
}

/// Error recovery utilities and classification
impl MediaForgeError {
    /// Determines if an error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            MediaForgeError::NetworkError(_) => true,
            MediaForgeError::TemporaryError(_) => true,
            MediaForgeError::YtDlpError(msg) => {
                // Common retryable yt-dlp errors
                msg.contains("network") || 
                msg.contains("timeout") ||
                msg.contains("connection") ||
                msg.contains("temporary") ||
                msg.contains("503") || // Service unavailable
                msg.contains("502") || // Bad gateway
                msg.contains("429")    // Too many requests
            },
            MediaForgeError::FFmpegError(msg) => {
                // Some FFmpeg errors might be retryable
                msg.contains("resource temporarily unavailable") ||
                msg.contains("interrupted system call")
            },
            MediaForgeError::FileSystemError(msg) => {
                // Some filesystem errors are temporary
                msg.contains("resource temporarily unavailable") ||
                msg.contains("device busy")
            },
            _ => false,
        }
    }
    
    /// Gets the recommended retry delay in seconds
    pub fn retry_delay(&self) -> u64 {
        match self {
            MediaForgeError::NetworkError(_) => 5,  // 5 seconds for network
            MediaForgeError::TemporaryError(_) => 3, // 3 seconds for temporary
            MediaForgeError::YtDlpError(msg) => {
                if msg.contains("429") { 30 } else { 10 } // Rate limit vs other
            },
            _ => 10, // Default 10 seconds
        }
    }
    
    /// Classifies system errors into appropriate MediaForgeError types
    pub fn from_system_error(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::PermissionDenied => {
                MediaForgeError::PermissionError(err.to_string())
            },
            std::io::ErrorKind::NotFound => {
                MediaForgeError::FileSystemError(format!("File not found: {}", err))
            },
            std::io::ErrorKind::ConnectionRefused | 
            std::io::ErrorKind::ConnectionReset |
            std::io::ErrorKind::ConnectionAborted |
            std::io::ErrorKind::TimedOut => {
                MediaForgeError::NetworkError(err.to_string())
            },
            std::io::ErrorKind::Interrupted => {
                MediaForgeError::TemporaryError(err.to_string())
            },
            std::io::ErrorKind::OutOfMemory => {
                MediaForgeError::ResourceExhausted(format!("Out of memory: {}", err))
            },
            _ => {
                if err.to_string().contains("No space left on device") {
                    MediaForgeError::DiskSpaceError(err.to_string())
                } else {
                    MediaForgeError::FileSystemError(err.to_string())
                }
            }
        }
    }
}

/// Retry configuration for error recovery
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: u64,        // Base delay in seconds
    pub max_delay: u64,         // Maximum delay in seconds
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: 2,
            max_delay: 60,
            exponential_backoff: true,
        }
    }
}

impl RetryConfig {
    pub fn for_network() -> Self {
        Self {
            max_attempts: 5,
            base_delay: 3,
            max_delay: 30,
            exponential_backoff: true,
        }
    }
    
    pub fn for_filesystem() -> Self {
        Self {
            max_attempts: 2,
            base_delay: 1,
            max_delay: 5,
            exponential_backoff: false,
        }
    }
    
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        if self.exponential_backoff {
            let delay = self.base_delay * (2_u64.pow(attempt.saturating_sub(1)));
            delay.min(self.max_delay)
        } else {
            self.base_delay
        }
    }
}

/// Async retry utility with exponential backoff
pub async fn retry_async<F, Fut, T>(
    config: RetryConfig,
    mut operation: F,
) -> Result<T, MediaForgeError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, MediaForgeError>>,
{
    let mut last_error = None;
    
    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                log::warn!("Attempt {} failed: {}", attempt, error);
                
                // Don't retry if error is not retryable
                if !error.is_retryable() {
                    return Err(error);
                }
                
                last_error = Some(error);
                
                // Don't wait after the last attempt
                if attempt < config.max_attempts {
                    let delay = config.calculate_delay(attempt);
                    log::info!("Retrying in {} seconds (attempt {}/{})", delay, attempt + 1, config.max_attempts);
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                }
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| {
        MediaForgeError::TemporaryError("All retry attempts exhausted".to_string())
    }))
}

/// Disk space and resource validation utilities
pub mod validation {
    use super::MediaForgeError;
    use std::path::Path;
    
    /// Minimum free space required (500MB)
    const MIN_FREE_SPACE_BYTES: u64 = 500 * 1024 * 1024;
    
    /// Validates available disk space for a given path
    pub async fn validate_disk_space(path: &Path, estimated_size_bytes: Option<u64>) -> Result<(), MediaForgeError> {
        match get_available_space(path).await {
            Ok(available_bytes) => {
                let required_bytes = estimated_size_bytes.unwrap_or(MIN_FREE_SPACE_BYTES);
                let total_required = required_bytes + MIN_FREE_SPACE_BYTES; // Buffer
                
                if available_bytes < total_required {
                    return Err(MediaForgeError::DiskSpaceError(
                        format!(
                            "Insufficient disk space. Available: {:.1}MB, Required: {:.1}MB", 
                            available_bytes as f64 / 1024.0 / 1024.0,
                            total_required as f64 / 1024.0 / 1024.0
                        )
                    ));
                }
                Ok(())
            }
            Err(e) => {
                log::warn!("Could not check disk space for {:?}: {}", path, e);
                // Don't fail the operation if we can't check disk space
                Ok(())
            }
        }
    }
    
    /// Gets available disk space for a path (cross-platform)
    async fn get_available_space(path: &Path) -> Result<u64, std::io::Error> {
        use std::process::Command;
        
        // Use `df` command on Unix-like systems (Linux, macOS)
        let output = Command::new("df")
            .arg("-B1") // Output in bytes
            .arg(path)
            .output()?;
            
        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get disk space information"
            ));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Parse df output - format: filesystem blocks used available use% mounted_on
        for line in stdout.lines().skip(1) { // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                if let Ok(available) = parts[3].parse::<u64>() {
                    return Ok(available);
                }
            }
        }
        
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Could not parse disk space information"
        ))
    }
    
    /// Validates write permissions for a directory
    pub async fn validate_write_permissions(path: &Path) -> Result<(), MediaForgeError> {
        use tokio::fs;
        
        if !path.exists() {
            // Try to create the directory
            if let Err(e) = fs::create_dir_all(path).await {
                return Err(MediaForgeError::PermissionError(
                    format!("Cannot create directory {:?}: {}", path, e)
                ));
            }
        }
        
        // Try to create a test file
        let test_file = path.join(".mediaforge_write_test");
        match fs::write(&test_file, b"test").await {
            Ok(()) => {
                // Clean up test file
                let _ = fs::remove_file(&test_file).await;
                Ok(())
            }
            Err(e) => Err(MediaForgeError::PermissionError(
                format!("Cannot write to directory {:?}: {}", path, e)
            ))
        }
    }
    
    /// Cleanup partial/temporary files on error
    pub async fn cleanup_on_error(file_path: &Path) -> Result<(), MediaForgeError> {
        use tokio::fs;
        
        if file_path.exists() {
            log::info!("Cleaning up partial file: {:?}", file_path);
            if let Err(e) = fs::remove_file(file_path).await {
                log::warn!("Failed to cleanup file {:?}: {}", file_path, e);
                // Don't propagate cleanup errors - they're not critical
            }
        }
        Ok(())
    }
}
