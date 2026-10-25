#!/bin/bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:---run}"
case "$MODE" in --run|--build-only|--install|--verify|--logs|--debug|--telemetry) ;; *) echo "Usage: $0 [--build-only|--install|--verify|--logs|--debug|--telemetry]" >&2; exit 2;; esac
cd "$ROOT_DIR"
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
FFMPEG_BINARY="$ROOT_DIR/src-tauri/binaries/ffmpeg-$HOST_TRIPLE"
if [ ! -x "$FFMPEG_BINARY" ]; then sh scripts/build-ffmpeg-lgpl.sh; fi
RUST_PROFILE="debug"
CARGO_PROFILE="dev"
SWIFT_PROFILE="debug"
if [ "${LOCALSCRIBE_RELEASE:-0}" = "1" ]; then
    RUST_PROFILE="release"; CARGO_PROFILE="release"; SWIFT_PROFILE="release"
fi
# The native worker has no web frontend; release builds must not depend on dist/.
TAURI_CONFIG='{"build":{"frontendDist":[]}}' cargo build --manifest-path src-tauri/Cargo.toml --profile "$CARGO_PROFILE"
swift build --package-path macos -c "$SWIFT_PROFILE"
SWIFT_BINARY_DIR="$(swift build --package-path macos -c "$SWIFT_PROFILE" --show-bin-path)"
APP_BUNDLE="$ROOT_DIR/macos/.build/LocalScribe.app"
STAGING_ROOT="$(mktemp -d "$ROOT_DIR/macos/.build/staging.XXXXXX")"
trap 'rm -rf "$STAGING_ROOT"' EXIT
STAGED_APP="$STAGING_ROOT/LocalScribe.app"
mkdir -p "$STAGED_APP/Contents/MacOS" "$STAGED_APP/Contents/Resources/licenses"
cp "$SWIFT_BINARY_DIR/LocalScribe" "$STAGED_APP/Contents/MacOS/LocalScribe"
cp "src-tauri/target/$RUST_PROFILE/localscribe" "$STAGED_APP/Contents/MacOS/localscribe-engine"
cp "$FFMPEG_BINARY" "$STAGED_APP/Contents/MacOS/ffmpeg"
cp src-tauri/binaries/ffmpeg.LICENSE src-tauri/binaries/ffmpeg.README src-tauri/binaries/ffmpeg.BUILD.txt "$STAGED_APP/Contents/Resources/licenses/"
cp src-tauri/licenses/* "$STAGED_APP/Contents/Resources/licenses/"
# Reuse the project's original icon; native packaging does not need a web build.
ICONSET="$STAGING_ROOT/LocalScribe.iconset"
mkdir -p "$ICONSET"
for SIZE in 16 32 128 256 512; do
    sips -z "$SIZE" "$SIZE" src-tauri/icons/icon.png --out "$ICONSET/icon_${SIZE}x${SIZE}.png" >/dev/null
    DOUBLE_SIZE=$((SIZE * 2))
    sips -z "$DOUBLE_SIZE" "$DOUBLE_SIZE" src-tauri/icons/icon.png --out "$ICONSET/icon_${SIZE}x${SIZE}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$STAGED_APP/Contents/Resources/LocalScribe.icns"
python3 - "$STAGED_APP" <<'PY'
import json,plistlib,sys
from pathlib import Path
app=Path(sys.argv[1]);version=json.loads(Path('package.json').read_text())['version']
info={'CFBundleExecutable':'LocalScribe','CFBundleIdentifier':'com.localscribe.menubar','CFBundleName':'LocalScribe','CFBundleDisplayName':'LocalScribe','CFBundlePackageType':'APPL','CFBundleShortVersionString':version,'CFBundleVersion':version,'LSMinimumSystemVersion':'13.0','LSUIElement':True,'NSPrincipalClass':'NSApplication','CFBundleIconFile':'LocalScribe.icns','NSHighResolutionCapable':True}
with (app/'Contents/Info.plist').open('wb') as file:plistlib.dump(info,file)
PY
for EXECUTABLE in ffmpeg localscribe-engine LocalScribe; do codesign --force --sign - "$STAGED_APP/Contents/MacOS/$EXECUTABLE"; done
codesign --force --sign - "$STAGED_APP"
codesign --verify --deep --strict "$STAGED_APP"
if [ "$MODE" != "--build-only" ]; then
    if pgrep -x LocalScribe >/dev/null; then
        osascript -e 'tell application id "com.localscribe.menubar" to quit'
        for _ in {1..50}; do
            if ! pgrep -x LocalScribe >/dev/null; then break; fi
            sleep 0.1
        done
        if pgrep -x LocalScribe >/dev/null; then
            echo "LocalScribe is still running; update cancelled to preserve active work." >&2
            exit 1
        fi
    fi
    python3 - "$APP_BUNDLE" "$ROOT_DIR" <<'PY'
import os,signal,subprocess,sys,time
from pathlib import Path
bundles=[Path(sys.argv[1]),Path('/Applications/LocalScribe.app'),Path(sys.argv[2])/'src-tauri/target/debug/bundle/macos/LocalScribe.app']
paths={str(app/'Contents/MacOS'/name) for app in bundles for name in ['LocalScribe','localscribe','localscribe-engine']}
for line in subprocess.check_output(['ps','-axo','pid=,comm='],text=True).splitlines():
    fields=line.strip().split(None,1)
    if len(fields)!=2 or fields[1] not in paths:continue
    pid=int(fields[0])
    try:os.kill(pid,signal.SIGTERM)
    except ProcessLookupError:continue
    for _ in range(50):
        try:os.kill(pid,0)
        except ProcessLookupError:break
        time.sleep(.1)
PY
fi
# Only a generated app inside .build is replaced here.
rm -rf "$APP_BUNDLE"
mv "$STAGED_APP" "$APP_BUNDLE"
if [ "$MODE" = "--build-only" ]; then echo "Built $APP_BUNDLE"; exit 0; fi
if [ "$MODE" = "--install" ]; then
    python3 - "$APP_BUNDLE" <<'PY'
from pathlib import Path
import os,subprocess,sys,uuid
source=Path(sys.argv[1]);target=Path('/Applications/LocalScribe.app')
stage=Path('/Applications/LocalScribe-stage-'+uuid.uuid4().hex[:8]+'.app')
backup=Path.home()/'.Trash'/('LocalScribe-before-SwiftUI-'+uuid.uuid4().hex[:8]+'.app')
subprocess.run(['ditto',str(source),str(stage)],check=True)
subprocess.run(['codesign','--verify','--deep','--strict',str(stage)],check=True)
if target.exists():os.rename(target,backup)
try:os.rename(stage,target)
except Exception:
    if backup.exists():os.rename(backup,target)
    raise
print('Installed:',target,'; previous app:',backup)
PY
    APP_BUNDLE="/Applications/LocalScribe.app"
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$APP_BUNDLE"
fi
if [ "$MODE" = "--debug" ]; then exec lldb -- "$APP_BUNDLE/Contents/MacOS/LocalScribe"; fi
open -n "$APP_BUNDLE"
if [ "$MODE" = "--verify" ]; then sleep 1; pgrep -x LocalScribe >/dev/null; fi
if [ "$MODE" = "--logs" ] || [ "$MODE" = "--telemetry" ]; then exec log stream --info --predicate 'process == "LocalScribe" OR process == "localscribe-engine"'; fi
