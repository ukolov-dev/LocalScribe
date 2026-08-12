# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LocalScribe is a macOS desktop application for local audio/video transcription using OpenAI's Whisper models. The app runs entirely locally—no cloud uploads, built with Tauri 2 + React frontend and Rust backend using `whisper-rs` (whisper.cpp bindings).

**User flow:** Drop or select audio/video file → Choose Whisper model → Select language → Optionally enable speaker labels → Click Transcribe → Edit and export result to TXT/SRT/VTT.

## Development Commands

### Frontend development
```bash
npm run dev          # Start Vite dev server on port 1420
npm run build        # Compile TypeScript and bundle frontend
npm run preview      # Preview production build
```

### Tauri development
```bash
npm run tauri dev    # Run full app in dev mode (hot reload)
npm run tauri build  # Build production .app bundle
```

### Sidecar preparation
```bash
npm run prepare-sidecars  # Copies FFmpeg binaries to resource dir
```

### Running tests
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Architecture

### Frontend-Backend Communication
The app uses Tauri's invoke/event pattern. All backend calls go through `src/lib/tauri.ts` which wraps `invoke()` calls:

**Commands (frontend → backend):**
- `list_models`, `download_model`, `delete_model`, `select_custom_model`
- `list_speaker_models`, `download_speaker_model`, `delete_speaker_model`, `select_custom_speaker_model`
- `transcribe_file`, `cancel_transcription`
- `export_transcript`

**Events (backend → frontend):**
- `model-download-progress` / `speaker-model-download-progress`
- `transcription-progress` - `{jobId, phase, progress}`
- `transcription-segment` - `{jobId, segment}`
- `transcription-error` - `{jobId, code, message}`

### Key Backend Modules

**`transcription.rs`** - Core transcription logic using `whisper-rs`
- Loads Whisper models with optional Metal GPU support
- Handles language detection and multilingual transcription
- Implements chunked processing for `Multilingual` mode (10-second chunks) to preserve mixed languages
- Special prompt injection for multilingual mode to preserve original languages
- Auto-retry on CPU if GPU transcription fails
- Fallback for `Auto` language: tries specific languages (Russian, English) if detection produces empty results
- Supports cancellation via `AtomicBool` flags
- Progress and segment callbacks emit events to frontend

**`audio.rs`** - Audio preprocessing
- Validates input file formats (aac, mp3, wav, m4a, mp4, mov, webm)
- Uses FFmpeg sidecar to convert to 16 kHz mono PCM WAV
- Reads WAV samples as f32 vectors for Whisper

**`diarization.rs`** - Speaker diarization (heuristic implementation)
- Extracts audio features: RMS, zero-crossing rate, pitch, frequency bands
- K-means clustering (2 speakers) with weighted feature normalization
- Post-processing: smooth isolated segments, fill gaps
- **Note:** This is a basic heuristic approach. Full embedding-based diarization using ECAPA-TDNN/TitaNet models is planned but not yet implemented.

**`models.rs`** - Whisper model management
- Two built-in models: `large-v3-turbo-q5_0` (~547 MB, recommended) and `large-v3-turbo` (~1.5 GB, best quality)
- Model specs defined as static arrays (MODEL_SPECS)
- Validates models by file size and optional SHA256
- Supports custom `.bin` models via settings

**`speaker_models.rs`** - Speaker diarization model management
- Supports: `speaker-lite`, `ecapa-tdnn`, `titanet-large`
- Custom ONNX speaker models
- Multi-file model support (e.g., separate embedding/encoder files)
- **Note:** Download/management is implemented, but embedding inference isn't connected to the diarization pipeline yet

**`downloader.rs`** - HTTP downloads with progress
- Streams to `.part` files, validates, then atomically renames
- SHA256 verification for integrity
- Emits progress events during download

### State Management

**Backend:**
- `AppState` manages active transcription jobs with cancel flags
- Settings persisted to Tauri app config store

**Frontend:**
- React state for models, speaker models, transcript, progress
- `useEffect` listeners for backend events
- Job-based cancellation with unique job IDs
- UI state machine: `loading` → `no-model` / `ready` → `downloading` / `transcribing` → `completed` / `error`

### GPU/CPU Fallback Strategy

Metal GPU is used by default for models < 1GB. Set environment variables:
- `LOCALSCRIBE_DISABLE_METAL=1` - force CPU
- `LOCALSCRIBE_FORCE_METAL=1` - force GPU

If GPU transcription fails (Whisper errors or empty transcript), the system automatically retries on CPU.

### Model Storage

- Models: `~/Library/Application Support/LocalScribe/models/`
- Cache/temp: `~/Library/Caches/com.localscribe.desktop/`

## Important Constraints

- No Python runtime—all transcription through Rust/whisper.cpp
- Metal support only on Apple Silicon (aarch64-apple-darwin)
- Minimum macOS 13.0
- All audio converted to 16 kHz mono before transcription
- Temporary WAV files cleaned up after transcription completes

## Type Safety

Shared types defined in `src-tauri/src/types.rs` are serialized with `serde` and must match frontend types in `src/types/localscribe.ts`. All structs use `camelCase` for JSON compatibility.

## Testing Strategy

- Unit tests in Rust modules (e.g., `diarization.rs` tests)
- Focus on feature extraction, clustering, timestamp formatting
- Integration testing through manual UI testing in dev mode

## Language Modes

The app supports several language modes with different trade-offs:

- **Auto** - Automatic language detection. Falls back to trying Russian and English separately if detection fails.
- **Multilingual** - Preserves mixed languages (e.g., Russian/English) using chunked processing (10-second chunks). Slower but more accurate for mixed content.
- **Multilingual Fast** - Faster multilingual mode, less stable on complex mixed recordings.
- **Specific languages** - Russian, English, German, French, Spanish, Italian, Portuguese, Chinese, Japanese.

For mixed Russian/English content, use **Multilingual Fast** for better accuracy.

## Current Limitations

1. **Speaker diarization** is currently heuristic-based (audio feature extraction + K-means clustering), not full embedding inference. ECAPA-TDNN and TitaNet models can be downloaded but the embedding inference pipeline isn't connected yet.

2. **Turbo Full** (large-v3-turbo) runs on CPU only for stability, making it noticeably slower than Turbo Q5.

3. **Multilingual** mode uses chunked processing which is slower than single-pass transcription.

4. **Production signing/notarization** for macOS distribution is not yet configured—current builds use ad-hoc signing.

## Recommended Settings

For typical usage with mixed Russian/English content:
- Model: `Turbo Q5` (large-v3-turbo-q5_0)
- Language: `Multilingual Fast`
- Threads: `Auto`
- Speakers: `Off` for maximum speed, `Auto` if speaker labels are needed

For maximum quality with simpler content:
- Model: `Turbo Full` (accept slower CPU-only processing)
- Language: Specific language (e.g., `Russian` or `English`)
