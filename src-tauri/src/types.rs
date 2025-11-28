use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadType {
    Single,
    Bulk,
    Playlist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaFormat {
    #[serde(rename = "mp4")]
    Mp4,
    #[serde(rename = "mp3")]
    Mp3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Downloading,
    Processing,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrimSettings {
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub urls: Vec<String>,
    pub download_type: DownloadType,
    pub format: MediaFormat,
    pub quality: Option<String>,
    pub audio_quality: Option<String>,
    pub download_path: String,
    pub trim: Option<TrimSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub task_id: String,
    pub name: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub error: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConversionType {
    Image,
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSettings {
    pub resolution: Option<String>,
    pub bitrate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub bitrate: Option<String>,
    pub sample_rate: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSettings {
    pub quality: Option<u32>,
    pub resize: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertRequest {
    pub input_files: Vec<PathBuf>,
    pub conversion_type: ConversionType,
    pub output_format: String,
    pub output_path: String,
    pub video_settings: Option<VideoSettings>,
    pub audio_settings: Option<AudioSettings>,
    pub image_settings: Option<ImageSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub has_ytdlp: bool,
    pub has_ffmpeg: bool,
    pub ytdlp_path: Option<String>,
    pub ffmpeg_path: Option<String>,
}
