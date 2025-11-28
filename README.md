# MediaForge

<div align="center">

**A production-grade desktop application for downloading YouTube videos/audio and converting media files**

Built with Tauri, React, TypeScript, and Rust • Optimized for Arch Linux with Hyprland

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.9-blue)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.77+-orange)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-18-blue)](https://reactjs.org/)

</div>

---

## 🚀 Features

### 📥 YouTube Downloader
- **Multiple download modes**: Single video, bulk downloads, full playlists
- **Format options**: MP4 (video) and MP3 (audio)
- **Quality selection**: 
  - Video: 360p, 480p, 720p HD, 1080p Full HD, 1440p 2K, 2160p 4K
  - Audio: 128, 192, 256, 320 kbps
- **Advanced features**:
  - Trim/cut videos by specifying start and end times
  - Custom download location
  - Concurrent downloads with individual control
  - Real-time progress tracking (percentage, speed, ETA)
  - Pause, resume, cancel, or remove downloads
  - Detailed error messages

### 🔄 Media Converter
- **Image conversion**: PNG, JPG, JPEG, WEBP, GIF, BMP, TIFF, ICO, SVG
- **Video conversion**: MP4, AVI, MOV, MKV, WEBM, FLV, WMV, MPEG, OGV
- **Audio conversion**: MP3, WAV, OGG, FLAC, AAC, M4A, WMA, OPUS
- **Features**:
  - Batch conversion (multiple files at once)
  - Quality and resolution controls
  - Custom output location
  - Real-time progress tracking

---

## 📋 System Requirements

### Operating System
- **Primary**: Arch Linux with Hyprland (optimized)
- **Supported**: Linux distributions with systemd

### Required Dependencies

**Core Development Tools:**
```bash
# Rust (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js & npm (v18+)
sudo pacman -S nodejs npm
```

**System Libraries (for Tauri):**
```bash
sudo pacman -S webkit2gtk base-devel curl wget file openssl \
               appmenu-gtk-module gtk3 libappindicator-gtk3 \
               librsvg libvips
```

**Media Tools:**
```bash
# yt-dlp (YouTube downloader)
sudo pacman -S yt-dlp

# FFmpeg (video/audio processing)
sudo pacman -S ffmpeg

# ImageMagick (image conversion)
sudo pacman -S imagemagick
```

---

## 🛠️ Installation

### 1. Clone the Repository
```bash
git clone https://github.com/ArindamTripathi619/MediaForge.git
cd MediaForge
```

### 2. Install Frontend Dependencies
```bash
npm install
```

### 3. Verify System Dependencies
```bash
# Check if all required tools are installed
which yt-dlp    # Should output: /usr/bin/yt-dlp
which ffmpeg    # Should output: /usr/bin/ffmpeg
which convert   # Should output: /usr/bin/convert (ImageMagick)
```

### 4. Build the Application

**Development Mode (with hot reload):**
```bash
npm run tauri:dev
```

**Production Build:**
```bash
npm run tauri:build
```

The compiled application will be in `src-tauri/target/release/bundle/`.

---

## 📖 Usage

### Downloading Videos/Audio

1. **Select Download Type**: Single, Bulk (multiple URLs), or Playlist
2. **Choose Format**: MP4 (video) or MP3 (audio)
3. **Enter URL(s)**: Paste YouTube URL(s)
4. **Select Quality**: Choose video quality or audio bitrate
5. **Choose Location**: Click the folder icon to select download directory
6. **Optional - Trim**: Enable trim and specify start/end times (e.g., 00:30 to 02:15)
7. **Start Download**: Monitor progress in the right sidebar

### Converting Media

1. **Select Media Type**: Image, Video, or Audio
2. **Add Files**: Drag & drop or click to browse (supports multiple files)
3. **Choose Output Format**: Select from available formats
4. **Configure Settings**: 
   - Video: Resolution, bitrate, codec
   - Audio: Bitrate, sample rate
   - Image: Quality, dimensions
5. **Select Output Location**: Choose where to save converted files
6. **Start Conversion**: Monitor progress in the queue

### Queue Management

- **Real-time updates**: See progress, speed, and ETA for each task
- **Individual controls**: Pause, cancel, or remove specific downloads
- **Clear completed**: Remove all finished tasks at once
- **Auto-refresh**: Updates every 2 seconds

---

## 🏗️ Architecture

### Frontend Stack
- **React 18**: Modern UI framework
- **TypeScript**: Type-safe development
- **Vite**: Fast development server and build tool
- **Tailwind CSS**: Utility-first styling
- **Lucide React**: Beautiful icon library
- **Tauri API**: Communication with Rust backend

### Backend Stack
- **Tauri v2**: Secure desktop application framework
- **Rust**: Safe, concurrent, and performant
- **Tokio**: Async runtime for I/O operations
- **DashMap**: Thread-safe concurrent task management
- **yt-dlp**: YouTube video/audio downloading
- **FFmpeg**: Media processing and conversion
- **ImageMagick**: Image format conversion

### Key Technical Features
- ✅ End-to-end type safety (Rust ↔ TypeScript)
- ✅ Real-time progress via event-driven updates
- ✅ Concurrent operations with async Rust
- ✅ Graceful error handling and recovery
- ✅ Memory-efficient file streaming
- ✅ Optimized for Hyprland (fullscreen and tiled modes)

---

## 📁 Project Structure

```
MediaForge/
├── src/                          # Frontend (React + TypeScript)
│   ├── components/
│   │   ├── DownloadSection.tsx  # YouTube download UI
│   │   ├── ConvertSection.tsx   # Media conversion UI
│   │   ├── DownloadQueue.tsx    # Task queue manager
│   │   ├── Header.tsx
│   │   └── Sidebar.tsx
│   ├── api/
│   │   └── tauri.ts            # Tauri API wrapper
│   ├── types/
│   │   └── tauri.ts            # TypeScript type definitions
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css
├── src-tauri/                    # Backend (Rust)
│   ├── src/
│   │   ├── lib.rs              # Main entry point
│   │   ├── commands.rs         # Tauri command handlers
│   │   ├── downloader.rs       # Download manager
│   │   ├── converter.rs        # Conversion manager
│   │   ├── system.rs           # System utilities
│   │   ├── types.rs            # Rust type definitions
│   │   └── error.rs            # Error handling
│   ├── Cargo.toml              # Rust dependencies
│   └── tauri.conf.json         # Tauri configuration
├── package.json
├── vite.config.ts
├── tailwind.config.js
├── LICENSE
└── README.md
```

---

## 🔧 Development

### Available Scripts

**Frontend:**
```bash
npm run dev        # Start Vite dev server
npm run build      # Build frontend
npm run lint       # Run ESLint
npm run typecheck  # TypeScript type checking
```

**Tauri:**
```bash
npm run tauri:dev    # Start app in development mode
npm run tauri:build  # Build production app
npm run tauri        # Run Tauri CLI commands
```

**Rust Backend:**
```bash
cd src-tauri
cargo build          # Build backend
cargo check          # Quick check without building
cargo test           # Run tests
cargo clean          # Clean build artifacts
```

---

## 🐛 Troubleshooting

### yt-dlp Not Found
```bash
# Install via pacman
sudo pacman -S yt-dlp

# OR via pip
pip3 install --user yt-dlp
export PATH="$HOME/.local/bin:$PATH"
```

### FFmpeg Not Found
```bash
sudo pacman -S ffmpeg
```

### ImageMagick Not Found
```bash
sudo pacman -S imagemagick
```

### Build Fails - Disk Space
```bash
# Clean cargo cache
cargo clean --manifest-path=src-tauri/Cargo.toml
rm -rf ~/.cargo/registry/cache

# Clean npm cache
npm cache clean --force
```

### WebKit Dependencies Missing
```bash
sudo pacman -S webkit2gtk base-devel
```

---

## 🎯 Performance

- **Concurrent Downloads**: Multiple simultaneous downloads using async Rust (Tokio)
- **Memory Efficiency**: Streams large files to avoid memory issues
- **Non-blocking UI**: Real-time progress updates without freezing
- **Error Recovery**: Graceful handling of network and system errors

---

## 🔒 Security

- **Sandboxed Execution**: Tauri provides secure sandboxing
- **No Remote Code**: No remote code execution vulnerabilities
- **Limited File Access**: File system access only to user-selected directories
- **Asset Protocol**: Secure resource loading

---

## 🤝 Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please test thoroughly on Arch Linux with Hyprland before submitting.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Credits

- **[Tauri](https://tauri.app/)**: Desktop application framework
- **[yt-dlp](https://github.com/yt-dlp/yt-dlp)**: YouTube downloading
- **[FFmpeg](https://ffmpeg.org/)**: Media processing
- **[ImageMagick](https://imagemagick.org/)**: Image conversion
- **[React](https://reactjs.org/)**: UI framework
- **[Rust](https://www.rust-lang.org/)**: Backend language

---

## 📞 Support

For issues, bugs, or feature requests:
- Open a [GitHub Issue](https://github.com/ArindamTripathi619/MediaForge/issues)
- Include:
  - System information (`neofetch`)
  - Error logs
  - Steps to reproduce
  - Expected vs actual behavior

---

<div align="center">

**Built with ❤️ for Arch Linux + Hyprland**

*Optimized for functionality and performance*

</div>
