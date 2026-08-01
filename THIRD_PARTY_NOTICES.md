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

The release sidecar is built from the unmodified official FFmpeg 8.1.2 source
archive. Its SHA256 is pinned in `scripts/build-ffmpeg-lgpl.sh`. The build turns
off GPL, nonfree, version-3-only, network, and optional external-library
components and verifies that the resulting configuration is LGPL v2.1 or
later.

Every binary LocalScribe release must include the exact FFmpeg source archive
as a release asset. The app bundle includes the LGPL license, source
provenance, checksum, and configure options.

Relevant upstream references:

- https://ffmpeg.org/legal.html
- https://ffmpeg.org/releases/ffmpeg-8.1.2.tar.xz

Local FFmpeg notice files are stored under:

- `src-tauri/binaries/ffmpeg.LICENSE`
- `src-tauri/binaries/ffmpeg.README`
- `src-tauri/binaries/ffmpeg.BUILD.txt`

Generated FFmpeg executable files should not be committed to the source repository.

## macOS and Apple APIs

LocalScribe targets macOS and uses platform capabilities through Tauri, WebKit, and system frameworks available on the user's machine.
