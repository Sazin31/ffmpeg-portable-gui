# Contributing to FFmpeg Portable GUI

Thank you for your interest in contributing to FFmpeg Portable GUI! 

## Code of Conduct
Please follow respectful and inclusive communication in all issues, pull requests, and discussions.

## How Can I Contribute?
- **Bug Reports**: Open an issue using the Bug Report template. Include your Windows version, FFmpeg version, and error output.
- **Feature Requests**: Suggest presets or transformations that benefit common desktop audio/video workflows.
- **Pull Requests**:
  1. Fork the repo and create your feature branch: `git checkout -b feature/my-new-tool`
  2. Make your modifications in `src/`.
  3. Ensure code formatting is clean: `cargo fmt` and `cargo check`.
  4. Run tests: `cargo test`.
  5. Commit with a clear commit message and submit a PR!

## Architecture Guidelines
- **Zero Console Popups**: Child processes must always specify `CREATE_NO_WINDOW` (`0x08000000`).
- **No `ffprobe`**: Keep the footprint small by parsing `ffmpeg -i` stderr for media metadata.
- **Safety First**: Never silently overwrite user files. Use `get_safe_destination_path`.
- **Pure Offline**: No telemetry, tracking, or network calls.
