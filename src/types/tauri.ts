export type DownloadType = 'Single' | 'Bulk' | 'Playlist';
export type MediaFormat = 'mp4' | 'mp3';
export type TaskStatus = 'Queued' | 'Downloading' | 'Processing' | 'Paused' | 'Completed' | 'Failed' | 'Cancelled';

export interface TrimSettings {
  start_time: string;
  end_time: string;
}

export interface DownloadRequest {
  urls: string[];
  download_type: DownloadType;
  format: MediaFormat;
  quality?: string;
  audio_quality?: string;
  download_path: string;
  trim?: TrimSettings;
}

export interface TaskProgress {
  task_id: string;
  name: string;
  status: TaskStatus;
  progress: number;
  speed?: string;
  eta?: string;
  error?: string;
  file_path?: string;
}

export type ConversionType = 'Image' | 'Video' | 'Audio';

export interface VideoSettings {
  resolution?: string;
  bitrate?: string;
}

export interface AudioSettings {
  bitrate?: string;
  sample_rate?: string;
}

export interface ImageSettings {
  quality?: number;
  resize?: string;
}

export interface ConvertRequest {
  input_files: string[];
  conversion_type: ConversionType;
  output_format: string;
  output_path: string;
  video_settings?: VideoSettings;
  audio_settings?: AudioSettings;
  image_settings?: ImageSettings;
}

export interface SystemInfo {
  has_ytdlp: boolean;
  has_ffmpeg: boolean;
  ytdlp_path?: string;
  ffmpeg_path?: string;
}
