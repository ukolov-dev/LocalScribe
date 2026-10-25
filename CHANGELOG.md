# Changelog

All notable changes to LocalScribe are documented here.

## [0.3.0] - 2026-10-25

### Added

- Native SwiftUI/AppKit menu-bar interface with an anchored panel, hidden launch,
  and no Dock icon. Drop a recording to start transcription automatically.
- Local neural speaker diarization through sherpa-onnx, Pyannote segmentation and
  3D-Speaker embeddings. Speaker count is inferred automatically without a
  two-person cap; speaker recognition is always enabled.
- Automatic SHA-256-verified speaker-model setup, isolated cancellable inference,
  and word-time alignment that retains speaker changes within a Whisper segment.
- Editable local draft restoration, TXT/Markdown export and native model management.
- Local meeting import/library integration with an authenticated loopback API.
- Swift regression tests and a reproducible public four-speaker inference smoke test.

### Changed

- macOS packaging now ships the native interface and a private Rust/Whisper worker;
  the React compatibility interface is retained in source.
- Existing models, settings and meeting data remain in their original storage.
- Updated GitHub Pages, download links and native interface illustration.

### Known limitations

- Ad-hoc signed Apple Silicon prerelease, not Developer ID signed or notarized.
- Speaker labels can be wrong for similar voices, noise, brief replies or overlap;
  uncertain speech is labeled unknown. This is not voice-based name identification.
- Clean-Mac validation and a diarization error-rate benchmark on real meetings
  remain outstanding.

## [0.1.2] - 2026-08-12

### Added

- Standalone AAC/ADTS audio input in the file picker, drag-and-drop flow, and
  local FFmpeg conversion pipeline.
- Automated AAC decoding coverage for the bundled LGPL-only FFmpeg sidecar.

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

[0.1.2]: https://github.com/ukolov-dev/LocalScribe/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/ukolov-dev/LocalScribe/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ukolov-dev/LocalScribe/releases/tag/v0.1.0

[0.3.0]: https://github.com/ukolov-dev/LocalScribe/compare/v0.1.2...v0.3.0
