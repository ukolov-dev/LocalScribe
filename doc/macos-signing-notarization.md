# macOS Signing and Notarization

LocalScribe is configured with:

- macOS minimum system version: `13.0`.
- Hardened runtime: enabled.
- Entitlements file: `src-tauri/Entitlements.plist`.

## Local ad-hoc signed build

```bash
npm run tauri build
```

This produces:

```text
src-tauri/target/release/bundle/macos/LocalScribe.app
```

The local build uses `signingIdentity = "-"` for ad-hoc signing. This is enough for local `codesign --verify`, but it is not a distribution signature and should not be treated as notarized.

## Developer ID release build

Use a Developer ID Application certificate installed in the login keychain.

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Example Inc (TEAMID)"
export APPLE_ID="apple-id@example.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAMID"

npm run tauri build
```

For CI, store these values as secrets and never commit them.

## GitHub release environment

The `Signed macOS release` workflow runs for `v*` tags, creates a draft
prerelease, builds the arm64 app and DMG, signs nested binaries, notarizes and
staples the artifacts, verifies them, and attaches the exact FFmpeg source.

Create a GitHub environment named `release` and add:

- `APPLE_CERTIFICATE` — base64-encoded Developer ID Application `.p12`.
- `APPLE_CERTIFICATE_PASSWORD` — password used when exporting the `.p12`.
- `KEYCHAIN_PASSWORD` — a dedicated temporary CI keychain password.
- `APPLE_ID` — Apple Developer account email.
- `APPLE_PASSWORD` — app-specific Apple ID password.
- `APPLE_TEAM_ID` — Apple Developer Team ID.

Export the certificate without line breaks:

```bash
openssl base64 -A -in DeveloperIDApplication.p12 -out certificate-base64.txt
```

After all secrets are configured and CI passes, create and push a tag matching
the version in `src-tauri/tauri.conf.json`, for example:

```bash
git tag -a v0.1.1 -m "LocalScribe v0.1.1"
git push origin v0.1.1
```

The workflow intentionally leaves the release as a draft prerelease so a clean
machine installation can be checked before publication.

## Manual verification

```bash
codesign --verify --deep --strict --verbose=2 src-tauri/target/release/bundle/macos/LocalScribe.app
```

After a Developer ID build and notarization, also run:

```bash
spctl --assess --type execute --verbose=4 src-tauri/target/release/bundle/macos/LocalScribe.app
```

Validate the stapled ticket on both the app and DMG:

```bash
xcrun stapler validate src-tauri/target/aarch64-apple-darwin/release/bundle/macos/LocalScribe.app
xcrun stapler validate src-tauri/target/aarch64-apple-darwin/release/bundle/dmg/LocalScribe_0.1.1_aarch64.dmg
```

## Notes

- The bundled ffmpeg binary must be covered in third-party license notices before distribution.
- Distribution outside your own machines requires a real Developer ID certificate and Apple notarization credentials.
- Keep `Entitlements.plist` minimal; add permissions only when a concrete macOS capability requires them.
