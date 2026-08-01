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

This keeps the committed FFmpeg license, README, and build record available
while excluding generated executables.

## FFmpeg

LocalScribe uses FFmpeg as a sidecar executable for audio/video conversion.

The project builds an unmodified FFmpeg 8.1.2 sidecar directly from the
official archive. `scripts/build-ffmpeg-lgpl.sh` pins the archive by SHA256 and
uses a closed build configuration:

- `--disable-gpl`
- `--disable-nonfree`
- `--disable-autodetect`
- `--disable-network`
- no optional external codec libraries

The build additionally verifies `CONFIG_GPL=0`, `CONFIG_NONFREE=0`, and
`CONFIG_VERSION3=0`, then rejects an executable whose embedded configure line
contains `--enable-gpl` or `--enable-nonfree`.

The sidecar is a separate executable invoked as a child process; LocalScribe
does not link FFmpeg libraries into its own executable. Binary releases must:

1. Include `ffmpeg.LICENSE`, `ffmpeg.README`, and `ffmpeg.BUILD.txt` in the app.
2. Attach the exact `ffmpeg-8.1.2.tar.xz` source archive beside the binary
   release.
3. Preserve the upstream copyright and license notices.
4. Make the reproducible build script available in the corresponding
   LocalScribe source tag.

Relevant upstream references:

- FFmpeg legal and compliance notes: https://ffmpeg.org/legal.html
- FFmpeg 8.1.2 source: https://ffmpeg.org/releases/ffmpeg-8.1.2.tar.xz

This engineering record is not a substitute for legal advice. Re-review the
exact binary and notices whenever the FFmpeg version or configure flags change.

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

- Run `npm run ffmpeg:build` and `npm run ffmpeg:verify`.
- Confirm the bundled executable still reports FFmpeg 8.1.2 and no GPL/nonfree flags.
- Confirm the exact FFmpeg source archive is attached to the release.
- Include third-party notices in the app bundle.
- Confirm model files are not bundled unless their redistribution terms are reviewed.
- Keep signing and notarization secrets outside the repository.
