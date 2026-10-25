# LocalScribe

**Private, offline audio and video transcription for Apple Silicon Macs.**

[![CI](https://github.com/ukolov-dev/LocalScribe/actions/workflows/ci.yml/badge.svg)](https://github.com/ukolov-dev/LocalScribe/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-2f6f62.svg)](LICENSE)
![macOS 13+](https://img.shields.io/badge/macOS-13%2B-111111?logo=apple)
![Apple Silicon](https://img.shields.io/badge/Apple%20Silicon-M1%20or%20newer-555555)
![SwiftUI](https://img.shields.io/badge/UI-SwiftUI-orange?logo=swift&logoColor=white)

LocalScribe is a privacy-first desktop speech-to-text app for macOS. It transcribes audio and video locally with Whisper/whisper.cpp, keeps media on your Mac, and produces editable transcripts with timestamps and automatic speaker labels in a native SwiftUI panel opened only from the macOS menu bar.

The macOS interface is SwiftUI hosted in an AppKit menu-bar panel. It launches without a Dock icon or a visible window.

> [!NOTE]
> LocalScribe is currently an alpha for Apple Silicon on macOS 13+. The [v0.3.0 binary](https://github.com/ukolov-dev/LocalScribe/releases/tag/v0.3.0) is an ad-hoc-signed prerelease: macOS Gatekeeper may require Control-clicking the app, choosing Open, and confirming once. A Developer ID signed and notarized build is planned.

## Why LocalScribe?

- **Offline transcription:** audio and video are processed on-device instead of being uploaded to a cloud transcription service.
- **Local Whisper models:** download supported GGML models or select a custom Whisper `.bin` file.
- **macOS tray panel:** drop a recording or choose a file to start immediately. Settings remain collapsed until needed. LocalScribe runs in the macOS menu bar without a Dock icon, with its interface hidden until the tray icon is clicked. Click the menu-bar icon to show or hide the panel, anchored below the icon and kept within the screen. Closing the window keeps transcription running in the tray; use Quit LocalScribe in the tray menu to exit.
- **Common media formats:** open standalone AAC/ADTS, WAV, MP3, M4A, MP4, MOV, and WebM files.
- **Useful output:** edit the transcript, keep timestamps, automatically identify speakers, and export to `TXT` or Markdown.
- **Multilingual speech recognition:** use automatic, multilingual, or language-specific decoding modes.
- **Native macOS interface:** SwiftUI + AppKit. The existing Rust/Whisper engine runs as a private background process without windows or WebViews.

## How It Works

1. Install or select a local Whisper model.
2. Optionally adjust language, decoding quality, and timestamps in Settings.
3. Drop one audio or video file into LocalScribe or click to choose one. Transcription starts automatically; if a model is missing, install it and the selected recording starts next.
4. Speaker detection always runs, including for imported meetings. Edit, copy, or export the result. Use Stop to cancel; Escape or the close button hides the panel while processing continues.

Speaker diarization uses local Pyannote segmentation and 3D-Speaker ERes2Net embeddings through sherpa-onnx. The number of voices is inferred automatically without a two-person cap. The 46 MB model pack downloads automatically on first use with pinned revisions and SHA-256 verification. A new drop is rejected while a recording is being processed. The latest transcript and edits are saved in a local owner-only draft and restored after relaunch.

After manual edits, TXT and Markdown export the edited text without the original segment timestamps. The editor is read-only while transcription runs.

## eXpress Meeting Integration

LocalScribe can accept recordings created by the companion eXpress desktop patch. Press `RnD транскрибация` in an active eXpress meeting, confirm that participants consent, then press `■ Стоп транскрибацию`. eXpress saves a local WebM file, opens LocalScribe, and places a versioned request in:

```text
~/Library/Application Support/com.localscribe.desktop/integration/inbox/
```

LocalScribe claims one request at a time, shows the imported recording in the normal transcription UI, and writes `<job-id>.localscribe.md` next to the source file, so separate jobs cannot overwrite each other. eXpress requests enable speaker detection and may include the meeting roster plus an active-speaker timeline. LocalScribe uses time overlap to map diarized speaker clusters to participant names, but only applies a name when the match is strong and unambiguous; otherwise it keeps `Speaker N`. The structured timestamped segments are saved under `integration/segments/` for the meeting UI. It also creates a durable meeting record under `integration/meetings/`; existing completed receipts are migrated automatically. Completed and failed receipts remain under the integration directory, and an interrupted request is recovered after restart.

While LocalScribe is running it exposes the meeting library through an authenticated loopback API at `http://127.0.0.1:17865/v1`. The API provides health, meeting-list, meeting-detail, and summary-update routes. A random 64-character bearer token is created in `integration/api-token` with owner-only permissions. The service binds only to loopback and does not enable browser CORS. Media and transcription stay local; only an explicit downstream integration such as the eXpress RND summary action may send transcript text to its configured LLM.

## Whisper Model Manager

Models are stored in macOS Application Support. Custom files are linked from their existing location rather than copied.

The native model manager supports downloads, removal and custom Whisper files. Existing models and settings remain under `~/Library/Application Support/com.localscribe.desktop/`.

Built-in downloads cover the useful stable Whisper.cpp profiles:

- `tiny-q5_1`, `base-q5_1`, and `small-q5_1` — compact multilingual options for faster transcription and lower memory use.
- `large-v3-turbo-q5_0` — recommended multilingual balance of speed, memory use, and quality.
- `large-v3-turbo-q8_0` and `large-v3-turbo` — higher-precision Turbo options.
- `medium-q5_0`, `large-v3-q5_0`, and `large-v3` — slower multilingual accuracy-focused options.
- `distil-large-v3.5` — fast long-form transcription for English audio only.
- Custom Whisper-compatible GGML `.bin` models.

Every built-in URL is pinned to an immutable Hugging Face revision. Downloads are checked against their exact file size and SHA-256 digest before installation.

Default model directory:

```text
~/Library/Application Support/com.localscribe.desktop/models/
```

## Requirements

- Apple Silicon Mac (M1 or newer recommended).
- macOS 13 or newer.
- Rust stable toolchain.
- Xcode Command Line Tools.

## Build and Run

On macOS, install Xcode command-line tools, Rust and Python 3. Then build and launch the native app:

```bash
./script/build_and_run.sh
```

Build without launching, or install to Applications (the previous app is moved to Trash):

```bash
./script/build_and_run.sh --build-only
./script/build_and_run.sh --install
```

The app bundle is `macos/.build/LocalScribe.app`. React, Node.js and a development web server are not needed by the native interface. The script builds the pinned LGPL-only FFmpeg sidecar from official source when missing; subsequent builds reuse it. Use `LOCALSCRIBE_RELEASE=1` for an optimized Rust/Swift build.

SwiftUI owns the panel, file drop, model management, transcript editor, clipboard and native open/save dialogs. AppKit owns the menu-bar item and on-demand panel. Rust runs with `--native-worker`, communicates through private JSON lines on stdin/stdout and exits when the parent closes its input. The worker clears all Tauri windows and does not create a tray icon or WebView. The legacy React/Tauri UI remains in source for compatibility and is not packaged as the macOS interface.

## Development and Tests

```bash
swift test --package-path macos
cargo test --manifest-path src-tauri/Cargo.toml
```

`./script/build_and_run.sh --verify` builds, launches and checks the process. `--logs` streams process logs; `--debug` starts the packaged app under LLDB.

Legacy frontend checks are still available with `npm ci`, `npm test` and `npm run build`. FFmpeg can be built and checked independently:

```bash
sh scripts/build-ffmpeg-lgpl.sh
sh scripts/verify-ffmpeg-sidecar.sh
```

## Privacy and Network Access

LocalScribe is designed for local-first transcription:

- Input audio and video files are not uploaded by the app.
- Transcription runs locally through Whisper/whisper.cpp.
- Downloaded models remain on the user's machine.
- No telemetry or cloud speech processing is enabled by default.
- Network access is used to download speech models and first-use speaker models from the configured Hugging Face endpoints.

## Current Status

The primary local transcription flow is implemented. Public binaries are currently ad-hoc signed prereleases; production distribution still needs Developer ID signing, notarization, and clean-Mac validation.

Known limitations:

- Speaker diarization can confuse similar voices, short interjections and overlapping speech. It assigns anonymous Speaker N labels within each recording; it does not identify a person by name or recover two simultaneous transcripts. Old unused speaker-model files remain on disk for compatibility; the active pipeline uses the Pyannote + 3D-Speaker pack.
- The full Turbo model can be slow when running on CPU.
- `SRT` and `VTT` export are planned; the current app exports `TXT` and `MD`.
- The v0.3.0 macOS prerelease is not Developer ID signed or notarized.
- Production Developer ID secrets are not configured in GitHub yet.
- Downloadable model licenses must be rechecked before the project ever bundles models directly.

## Security

- The native UI uses standard macOS file dialogs. Its private worker accepts bounded JSON requests through inherited pipes, not a public transcription endpoint.
- Legacy Tauri capabilities and CSP remain scoped to the compatibility interface.
- Checksums are configured for built-in downloadable model files where available.
- Generated binaries, downloaded models, local settings, and secrets are excluded from Git.

Please report security issues according to [SECURITY.md](SECURITY.md).

## Documentation

- [Roadmap](ROADMAP.md)
- [Changelog](CHANGELOG.md)
- [Product specification](doc/LocalScribe.md)
- [Diarization plan](doc/diarization-plan.md)
- [Licensing notes](doc/licensing.md)
- [Smoke-test guide](doc/smoke-test.md)
- [Release checklist](doc/release-checklist.md)
- [macOS signing and notarization](doc/macos-signing-notarization.md)
- [Launch kit](doc/launch-kit.md)

## Contributing

Issues, test results, and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md) before submitting a change.

If LocalScribe is useful to you, consider starring the repository — it helps other people discover the project.

## License

LocalScribe source code is available under the [MIT License](LICENSE).

Third-party dependencies, FFmpeg binaries, and downloadable models have their own licenses and distribution terms. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
