# LocalScribe QA / Build / Release Checklist

## MVP verification

- `cargo check` passes in `src-tauri`.
- `cargo test` passes in `src-tauri`.
- `npm run build` passes.
- `npm run ffmpeg:build` builds FFmpeg 8.1.2 with LGPL-only configuration.
- `npm run ffmpeg:verify` decodes all six committed media fixtures.
- `npm run tauri build` produces `LocalScribe.app`.
- `src-tauri/target/release/bundle/macos/LocalScribe.app/Contents/MacOS/ffmpeg -version` works.
- Model download is tested from UI with `large-v3-turbo-q5_0`.
- Failed model download leaves only `.part` file and can be retried.
- `mp3`, `wav`, `m4a`, `mp4`, `mov`, and `webm` inputs are smoke-tested.
- Russian speech is tested with `large-v3-turbo-q5_0`.
- Exported `TXT` and `MD` files are opened and timestamp formats are checked.
- CLI smoke test in `doc/smoke-test.md` passes.
- Cancellation is tested during a long transcription.
- Temporary WAV files are removed after success, error, and cancellation.

## Release blockers before distribution

- Review and approve the generated production icon set.
- Attach `ffmpeg-8.1.2-source.tar.xz` to the GitHub release and verify all three FFmpeg notice files are bundled.
- Verify pinned model SHA256 values against upstream before each release.
- Test on clean macOS 13, 14, and 15 Apple Silicon machines.
- Add real Developer ID signing identity and notarization secrets for distribution.
- Verify Metal transcription in the signed `.app`; CPU smoke passed locally, but the local headless Metal smoke failed in `ggml_metal_buffer_init`.
- Add crash/error logging that stays local by default.
- Add UI smoke tests for model manager, transcription flow, and exports.

## Signed GitHub release

- Version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` matches the release tag.
- GitHub `release` environment contains all secrets documented in `doc/macos-signing-notarization.md`.
- Push the matching `v*` tag only after CI passes on the release commit.
- Confirm the release workflow verifies `codesign`, Gatekeeper, and stapled notarization tickets.
- Confirm the draft release contains a DMG, app archive, and `ffmpeg-8.1.2-source.tar.xz`.
- Install the DMG on a clean Apple Silicon Mac and run the full transcription smoke test before publishing the draft.
