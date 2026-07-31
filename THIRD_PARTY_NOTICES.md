# Third-Party Notices

This file summarizes notable third-party components used by LocalScribe. It is not a complete dependency license report.

For release-oriented licensing notes, see `doc/licensing.md`.

## Runtime and Frameworks

LocalScribe uses Tauri, React, TypeScript, Vite, Rust crates, and related npm packages. Their exact versions are tracked in:

- `package-lock.json`
- `src-tauri/Cargo.lock`

Review the upstream licenses before distributing binaries.

## Whisper

LocalScribe uses `whisper-rs`, which provides Rust bindings around `whisper.cpp`.

Whisper-compatible model files are not committed to this repository. The app can download configured GGML model files from Hugging Face when requested by the user. Those model files have separate upstream licenses and distribution terms.

Configured ASR model sources are defined in `src-tauri/src/models.rs`.

Relevant upstream references:

- https://github.com/ggerganov/whisper.cpp/blob/master/LICENSE
- https://github.com/openai/whisper/blob/main/LICENSE
- https://huggingface.co/ggerganov/whisper.cpp

## Speaker Models

LocalScribe can manage optional speaker model files. These files are not committed to this repository and have separate upstream licenses and distribution terms.

Configured speaker model sources are defined in `src-tauri/src/speaker_models.rs`.

## FFmpeg

LocalScribe uses FFmpeg as a sidecar binary for audio/video conversion.

The current development setup uses the npm package `ffmpeg-static` to prepare the FFmpeg sidecar. The package metadata in this project identifies `ffmpeg-static` as `GPL-3.0-or-later`.

Do not distribute packaged app binaries until FFmpeg licensing obligations have been reviewed for the exact binary being shipped.

Relevant upstream references:

- https://ffmpeg.org/legal.html
- https://github.com/eugeneware/ffmpeg-static

Local FFmpeg notice files are stored under:

- `src-tauri/binaries/ffmpeg.LICENSE`
- `src-tauri/binaries/ffmpeg.README`

Generated FFmpeg executable files should not be committed to the source repository.

## macOS and Apple APIs

LocalScribe targets macOS and uses platform capabilities through Tauri, WebKit, and system frameworks available on the user's machine.
