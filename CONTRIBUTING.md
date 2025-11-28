# Contributing to MediaForge

First off, thank you for considering contributing to MediaForge! 🎉

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the existing issues to avoid duplicates. When you create a bug report, include as many details as possible:

**Bug Report Template:**
```
**System Information:**
- OS: [e.g., Arch Linux]
- Window Manager: [e.g., Hyprland]
- MediaForge Version: [e.g., 1.0.0]

**Describe the bug:**
A clear and concise description of what the bug is.

**To Reproduce:**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '...'
3. See error

**Expected behavior:**
A clear description of what you expected to happen.

**Screenshots:**
If applicable, add screenshots to help explain your problem.

**Error Logs:**
Include any error messages from the console or logs.
```

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, include:

- **Clear title and description** of the suggested enhancement
- **Use case** - why would this be useful?
- **Possible implementation** (optional)

### Pull Requests

1. **Fork the repository** and create your branch from `main`
   ```bash
   git checkout -b feature/amazing-feature
   ```

2. **Make your changes** following the code style guidelines below

3. **Test thoroughly** on Arch Linux with Hyprland
   - Test in fullscreen mode
   - Test in tiled mode
   - Test edge cases

4. **Commit your changes**
   ```bash
   git commit -m 'Add amazing feature'
   ```

5. **Push to your fork**
   ```bash
   git push origin feature/amazing-feature
   ```

6. **Open a Pull Request** with a clear title and description

## Code Style Guidelines

### Rust (Backend)

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Write doc comments for public APIs
- Keep functions small and focused

```rust
/// Downloads a video from YouTube with the specified options.
///
/// # Arguments
/// * `url` - The YouTube video URL
/// * `format` - The desired output format (mp4/mp3)
///
/// # Returns
/// Returns a `Result` with the task ID or an error
pub async fn download_video(url: &str, format: DownloadFormat) -> Result<String> {
    // Implementation
}
```

### TypeScript/React (Frontend)

- Use TypeScript for all new code
- Follow React best practices
- Use functional components with hooks
- Keep components small and reusable
- Use meaningful variable names

```typescript
// Good
interface DownloadSectionProps {
  onDownloadStart: (options: DownloadOptions) => void;
  isLoading: boolean;
}

// Component structure
export function DownloadSection({ onDownloadStart, isLoading }: DownloadSectionProps) {
  // Implementation
}
```

### Commit Message Guidelines

Use clear and descriptive commit messages:

```
feat: add support for 8K video downloads
fix: resolve progress bar animation glitch
docs: update installation instructions for Fedora
refactor: simplify download queue logic
test: add unit tests for converter module
chore: update dependencies
```

**Prefixes:**
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks
- `perf:` - Performance improvements

## Development Setup

### Prerequisites

```bash
# System dependencies
sudo pacman -S nodejs npm rust webkit2gtk base-devel \
               yt-dlp ffmpeg imagemagick

# Install project dependencies
npm install
```

### Running Development Environment

```bash
# Start development server with hot reload
npm run tauri:dev

# Run frontend only
npm run dev

# Run type checking
npm run typecheck

# Run linting
npm run lint
```

### Testing

```bash
# Rust backend tests
cd src-tauri
cargo test

# Frontend tests (if implemented)
npm test
```

## Project Structure

Familiarize yourself with the codebase:

```
MediaForge/
├── src/                    # Frontend (React + TypeScript)
│   ├── components/         # React components
│   ├── api/               # API layer
│   └── types/             # TypeScript types
├── src-tauri/              # Backend (Rust)
│   └── src/
│       ├── commands.rs    # Tauri command handlers
│       ├── downloader.rs  # Download logic
│       ├── converter.rs   # Conversion logic
│       └── ...
└── docs/                   # Development documentation
```

## Areas That Need Help

- [ ] Unit tests for Rust modules
- [ ] Integration tests
- [ ] Support for more video platforms
- [ ] Internationalization (i18n)
- [ ] Custom FFmpeg options
- [ ] Download history feature
- [ ] Subtitle download support
- [ ] Video thumbnail preview

## Questions?

Feel free to open an issue with the `question` label if you have any questions about contributing!

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers
- Focus on what's best for the project
- Accept constructive criticism gracefully

---

Thank you for contributing to MediaForge! 🚀
