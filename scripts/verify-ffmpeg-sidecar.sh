#!/bin/sh

set -eu

host_triple() {
  case "$(uname -m)" in
    arm64) echo "aarch64-apple-darwin" ;;
    x86_64) echo "x86_64-apple-darwin" ;;
    *)
      echo "Unsupported macOS architecture: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)
TARGET_TRIPLE=${TAURI_TARGET_TRIPLE:-${TAURI_ENV_TARGET_TRIPLE:-$(host_triple)}}
FFMPEG_PATH="${PROJECT_ROOT}/src-tauri/binaries/ffmpeg-${TARGET_TRIPLE}"
FIXTURE_DIR="${PROJECT_ROOT}/src-tauri/tests/fixtures/media"
TEMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/localscribe-ffmpeg.XXXXXX")
trap 'rm -rf "${TEMP_DIR}"' EXIT HUP INT TERM

if [ ! -x "${FFMPEG_PATH}" ]; then
  echo "Missing FFmpeg sidecar: ${FFMPEG_PATH}" >&2
  exit 1
fi

VERSION_OUTPUT=$("${FFMPEG_PATH}" -version 2>&1)
echo "${VERSION_OUTPUT}" | grep -F "ffmpeg version 8.1.2" >/dev/null
if echo "${VERSION_OUTPUT}" | grep -E -- '--enable-(gpl|nonfree)' >/dev/null; then
  echo "FFmpeg sidecar unexpectedly enables GPL or nonfree code." >&2
  exit 1
fi

for INPUT_PATH in "${FIXTURE_DIR}"/*; do
  [ -f "${INPUT_PATH}" ] || continue
  FILE_NAME=$(basename -- "${INPUT_PATH}")
  OUTPUT_PATH="${TEMP_DIR}/${FILE_NAME}.wav"

  "${FFMPEG_PATH}" \
    -y \
    -hide_banner \
    -loglevel error \
    -i "${INPUT_PATH}" \
    -map 0:a:0 \
    -vn \
    -ac 1 \
    -ar 16000 \
    -c:a pcm_s16le \
    "${OUTPUT_PATH}"

  test -s "${OUTPUT_PATH}"
  echo "Decoded ${FILE_NAME}"
done

echo "FFmpeg sidecar format verification passed."
