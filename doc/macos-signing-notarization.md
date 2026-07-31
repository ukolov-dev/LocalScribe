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

## Manual verification

```bash
codesign --verify --deep --strict --verbose=2 src-tauri/target/release/bundle/macos/LocalScribe.app
```

After a Developer ID build and notarization, also run:

```bash
spctl --assess --type execute --verbose=4 src-tauri/target/release/bundle/macos/LocalScribe.app
```

## Notes

- The bundled ffmpeg binary must be covered in third-party license notices before distribution.
- Distribution outside your own machines requires a real Developer ID certificate and Apple notarization credentials.
- Keep `Entitlements.plist` minimal; add permissions only when a concrete macOS capability requires them.
