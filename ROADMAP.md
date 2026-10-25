# LocalScribe Roadmap

LocalScribe is an early privacy-first macOS transcription app. This roadmap is
directional; confirmed work is tracked in GitHub issues and milestones.

## Shipped in 0.3.0

- Native SwiftUI menu-bar panel with automatic file-drop transcription.
- Always-on local neural diarization with automatic speaker count.
- Local draft restoration, native model management and meeting import/export.

## Now — reliable distribution

- Publish clearly labeled unsigned Apple Silicon prereleases.
- Publish a signed and notarized Apple Silicon DMG.
- Validate installation and Metal transcription on clean macOS 13, 14, and 15.
- Keep the LGPL-only FFmpeg build reproducible and source-complete.
- Add local-only crash diagnostics and clearer recovery messages.
- Add automated UI smoke coverage for model management, transcription, and export.

## Next — better transcript workflows

- Add SRT and VTT subtitle export.
- Improve transcript search, correction, and timestamp navigation.
- Add configurable paragraph and sentence formatting.
- Improve progress estimates and cancellation behavior for long recordings.
- Expand synthetic media and multilingual regression coverage.

## Later — higher-quality speaker workflows

- Benchmark diarization on real meetings, including noise and overlapping voices.
- Add speaker renaming and merge/split controls.
- Explore optional custom vocabulary and prompt presets.
- Evaluate Intel Mac and additional platform support based on maintainer capacity.

## Principles

- Audio, video, and transcripts stay local by default.
- Network access is limited to speech-model downloads and automatic first-use speaker-model setup.
- Prerelease signing status is explicit; production artifacts will be Developer ID signed, verifiable, and accompanied by source and notices.
- Quality and privacy take priority over feature count.

See an opportunity to help? Open a focused feature request or comment on an
existing issue before starting a large change.
