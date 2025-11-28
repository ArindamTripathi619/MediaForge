# Changelog

All notable changes to MediaForge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-11-28

### Added
- **YouTube Downloader**
  - Single video/audio download
  - Bulk download support (multiple URLs)
  - Playlist download support
  - Format selection (MP4 for video, MP3 for audio)
  - Quality selection (360p to 4K for video, 128-320 kbps for audio)
  - Trim/cut functionality with start and end time specification
  - Custom download location selection
  - Concurrent downloads with individual control
  - Real-time progress tracking (percentage, speed, ETA)
  - Pause, resume, cancel operations
  - Detailed error messages

- **Media Converter**
  - Image conversion (PNG, JPG, WEBP, GIF, BMP, TIFF, ICO, SVG)
  - Video conversion (MP4, AVI, MOV, MKV, WEBM, FLV, WMV, MPEG, OGV)
  - Audio conversion (MP3, WAV, OGG, FLAC, AAC, M4A, WMA, OPUS)
  - Batch conversion support
  - Quality and resolution controls
  - Real-time progress tracking

- **User Interface**
  - Modern dark theme with gradient backgrounds
  - Tab-based navigation (Download/Convert)
  - Real-time task queue sidebar
  - Progress bars with animations
  - Status indicators
  - Responsive layout optimized for Hyprland (fullscreen and tiled modes)

- **Technical Features**
  - Tauri v2 framework for security and performance
  - Rust backend with async operations (Tokio)
  - React 18 frontend with TypeScript
  - End-to-end type safety
  - Event-driven architecture for real-time updates
  - Concurrent task management with DashMap
  - Graceful error handling

### Technical Stack
- **Backend**: Rust, Tauri 2.9, Tokio, yt-dlp, FFmpeg, ImageMagick
- **Frontend**: React 18, TypeScript, Vite, Tailwind CSS, Lucide React
- **Platform**: Arch Linux with Hyprland window manager

### Notes
- Optimized specifically for Arch Linux with Hyprland
- Works in both fullscreen and tiled window modes
- Minimum resolution: 1200x700
- Default resolution: 1400x900

---

## Future Releases

### [Unreleased]
Ideas for future versions:
- Download history tracking
- Keyboard shortcuts
- System tray integration
- Dark/light theme toggle
- Subtitle download support
- Advanced FFmpeg options
- Multi-language support (i18n)
- Support for more video platforms
- Video thumbnail preview
- Download scheduling

---

[1.0.0]: https://github.com/ArindamTripathi619/MediaForge/releases/tag/v1.0.0
