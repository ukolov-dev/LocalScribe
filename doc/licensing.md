# Licensing Notes

This document is an engineering note for open-source publication. It is not legal advice.

## Project License

LocalScribe's original source code is licensed under the MIT License. See `LICENSE`.

The MIT License covers this repository's own source code. It does not relicense third-party dependencies, FFmpeg binaries, downloadable Whisper models, or optional speaker models.

The project metadata declares MIT in:

- `package.json`
- `src-tauri/Cargo.toml`

## Source Repository Policy

The public GitHub repository should be source-first:

- Commit source code, docs, lockfiles, icons, and license notices.
- Do not commit generated FFmpeg executables.
- Do not commit downloaded Whisper or speaker model files.
- Do not commit local signing credentials, notarization credentials, or app-specific passwords.

The current `.gitignore` excludes generated FFmpeg sidecar binaries via:

```gitignore
src-tauri/binaries/ffmpeg-*
```

This keeps `src-tauri/binaries/ffmpeg.LICENSE` and `src-tauri/binaries/ffmpeg.README` available while excluding generated executables.

## FFmpeg

LocalScribe uses FFmpeg as a sidecar executable for audio/video conversion.

Relevant upstream references:

- FFmpeg legal notes: https://ffmpeg.org/legal.html
- `ffmpeg-static` package metadata: https://github.com/eugeneware/ffmpeg-static

Important practical point: the current development flow uses the npm package `ffmpeg-static`, whose package metadata identifies it as `GPL-3.0-or-later`.

For source-only GitHub publication, this is documented as a development/build dependency. For packaged app distribution, do not publish bundled `.app`, `.dmg`, or `.zip` artifacts until the exact FFmpeg binary and its obligations have been reviewed.

Reasonable binary distribution paths:

1. Keep public releases source-only until licensing is reviewed.
2. Ship a GPL-compatible binary release and satisfy the corresponding GPL obligations.
3. Replace the bundled binary with a verified LGPL-only FFmpeg build and satisfy LGPL obligations.
4. Do not bundle FFmpeg; require the user to install or select a local FFmpeg executable.

Do not use an FFmpeg build configured with nonfree components for public redistribution.

## Whisper and ASR Models

LocalScribe uses `whisper-rs` and `whisper.cpp` bindings for local transcription.

Relevant upstream references:

- `whisper.cpp` license: https://github.com/ggerganov/whisper.cpp/blob/master/LICENSE
- OpenAI Whisper license: https://github.com/openai/whisper/blob/main/LICENSE
- GGML model repository metadata: https://huggingface.co/ggerganov/whisper.cpp

The configured ASR model files are downloaded by users at runtime and are not committed to this repository. Before distributing model files yourself, review the license and terms for the exact model artifact.

## Speaker Models

Optional speaker model files are also downloaded or selected by users and are not committed to this repository.

Before enabling public binary releases that preinstall or redistribute speaker models, review the upstream license for each exact model file listed in `src-tauri/src/speaker_models.rs`.

## Release Gate

Before publishing binary releases:

- Confirm the exact FFmpeg binary license and build configuration.
- Confirm whether the release bundles FFmpeg or only downloads/prepares it locally.
- Include third-party notices in the app bundle.
- Confirm model files are not bundled unless their redistribution terms are reviewed.
- Keep signing and notarization secrets outside the repository.
