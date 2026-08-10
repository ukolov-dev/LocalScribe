# LocalScribe Roadmap

LocalScribe is an early privacy-first macOS transcription app. This roadmap is
directional; confirmed work is tracked in GitHub issues and milestones.

## Now — reliable first binary release

- Publish the clearly labeled unsigned v0.1.1 Apple Silicon prerelease.
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

- Replace heuristic speaker labels with a reviewed local diarization pipeline.
- Add speaker renaming and merge/split controls.
- Explore optional custom vocabulary and prompt presets.
- Evaluate Intel Mac and additional platform support based on maintainer capacity.

## Principles

- Audio, video, and transcripts stay local by default.
- Network access is explicit and limited to user-requested downloads.
- Prerelease signing status is explicit; production artifacts will be Developer ID signed, verifiable, and accompanied by source and notices.
- Quality and privacy take priority over feature count.

See an opportunity to help? Open a focused feature request or comment on an
existing issue before starting a large change.
