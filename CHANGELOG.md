# Changelog

All notable changes to LocalScribe are documented here.

## [0.1.1] - 2026-08-10

### Added

- Reproducible FFmpeg 8.1.2 sidecar build pinned to the official source SHA256.
- Automated decoding checks for WAV, MP3, M4A, MP4, MOV, and WebM.
- Ad-hoc-signed arm64 GitHub prerelease workflow with DMG packaging and explicit unsigned-build disclosure.
- Exact FFmpeg source attachment and bundled LGPL compliance records.
- Stable Whisper.cpp download profiles from Tiny Q5 through Large v3 Full, plus English-only Distil Large v3.5.
- Immutable Hugging Face revisions, exact-size validation, and SHA-256 verification for every built-in model download.

### Changed

- Replaced the GPL/nonfree `ffmpeg-static` dependency with an LGPL v2.1+
  configuration that disables GPL, nonfree, version-3-only, network, and
  optional external-library components.

## [0.1.0] - 2026-07-30

### Added

- Initial source-only alpha release with local Whisper transcription,
  timestamps, model management, transcript editing, and TXT/Markdown export.

[0.1.1]: https://github.com/ukolov-dev/LocalScribe/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ukolov-dev/LocalScribe/releases/tag/v0.1.0
