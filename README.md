# LocalScribe

LocalScribe is a macOS desktop app for local audio and video transcription. It uses Tauri 2, React, TypeScript, Rust, and `whisper-rs`/`whisper.cpp` so media files stay on the user's machine.

The current target is Apple Silicon macOS 13+.

## Privacy Model

LocalScribe is designed for local-first transcription:

- Audio and video files are processed locally.
- Transcription input files are not uploaded by the app.
- Downloaded models are stored on the user's machine.
- The app should not add telemetry or cloud processing by default.

## Features

- Local transcription without cloud uploads.
- Whisper model manager with downloads and custom `.bin` model selection.
- Audio/video input through drag and drop or the native file picker.
- Language modes for auto, multilingual, and specific languages.
- Optional speaker labels using the current heuristic diarization pipeline.
- Export to `TXT` and `MD`.
- FFmpeg sidecar conversion to 16 kHz mono PCM WAV.

## Current Status

This is an early desktop application. The main transcription flow is implemented, but release packaging still needs hardening before public binary distribution.

Known limitations:

- Speaker diarization is heuristic-based. The app can manage speaker model downloads, but embedding inference is not connected to the pipeline yet.
- The full Turbo model runs on CPU for stability and can be slow.
- `SRT` and `VTT` export are planned, but the current app exports `TXT` and `MD`.
- Production Developer ID signing and notarization are not configured.
- FFmpeg and model licensing must be reviewed before distributing packaged binaries.

## Requirements

- macOS 13 or newer.
- Apple Silicon Mac recommended.
- Node.js 18+.
- Rust stable toolchain.
- Xcode Command Line Tools.

## Quick Start

Install dependencies:

```bash
npm ci
```

Run the Tauri app in development mode:

```bash
npm run tauri dev
```

## Development Commands

Build the frontend:

```bash
npm run build
```

Build the macOS app bundle:

```bash
npm run tauri build
```

Run Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Run a fast Rust compile check:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## Models

LocalScribe downloads Whisper-compatible GGML models from Hugging Face when requested by the user. Model files are not stored in this repository.

Default app model directory:

```text
~/Library/Application Support/LocalScribe/models/
```

Supported built-in models:

- `large-v3-turbo-q5_0` - recommended for performance.
- `large-v3-turbo` - higher quality, slower.
- Custom `.bin` model selected by the user.

## Network Access

The app is designed to keep user media local. Network access is used for model downloads from configured model URLs, currently Hugging Face model endpoints defined in `src-tauri/src/models.rs` and `src-tauri/src/speaker_models.rs`.

Transcription input files are not uploaded by the app.

## Security Notes

- Tauri capabilities are intentionally narrow and currently expose core defaults plus native open/save dialogs.
- The desktop Content Security Policy is configured in `src-tauri/tauri.conf.json`.
- Checksums are configured for built-in downloadable model files where available.
- Do not commit local secrets, downloaded models, or generated binaries.

## Repository Hygiene

Do not commit generated or local artifacts:

- `node_modules/`
- `dist/`
- `src-tauri/target/`
- generated FFmpeg sidecar binaries
- local settings and secrets
- downloaded models

## Documentation

Additional project notes live in `doc/`:

- `doc/LocalScribe.md` - original product spec.
- `doc/licensing.md` - practical licensing notes for source and binary releases.
- `doc/smoke-test.md` - manual smoke test flow.
- `doc/release-checklist.md` - release readiness checklist.
- `doc/macos-signing-notarization.md` - macOS signing notes.

## License

LocalScribe source code is licensed under the MIT License. See `LICENSE`.

Third-party dependencies, FFmpeg binaries, and downloadable models have their own licenses and distribution terms. See `THIRD_PARTY_NOTICES.md`.
