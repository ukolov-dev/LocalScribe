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

FFMPEG_VERSION="8.1.2"
FFMPEG_ARCHIVE="ffmpeg-${FFMPEG_VERSION}.tar.xz"
FFMPEG_URL="https://ffmpeg.org/releases/${FFMPEG_ARCHIVE}"
FFMPEG_SHA256="464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c"

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PROJECT_ROOT=$(CDPATH= cd -- "${SCRIPT_DIR}/.." && pwd)
TARGET_TRIPLE=${TAURI_TARGET_TRIPLE:-${TAURI_ENV_TARGET_TRIPLE:-$(host_triple)}}
CACHE_ROOT="${PROJECT_ROOT}/src-tauri/target/ffmpeg-lgpl"
DOWNLOAD_DIR="${CACHE_ROOT}/downloads"
SOURCE_DIR="${CACHE_ROOT}/source/ffmpeg-${FFMPEG_VERSION}"
BUILD_DIR="${CACHE_ROOT}/build/${TARGET_TRIPLE}"
OUTPUT_PATH="${PROJECT_ROOT}/src-tauri/binaries/ffmpeg-${TARGET_TRIPLE}"
ARCHIVE_PATH="${DOWNLOAD_DIR}/${FFMPEG_ARCHIVE}"

case "${TARGET_TRIPLE}" in
  aarch64-apple-darwin)
    FFMPEG_ARCH="arm64"
    CLANG_ARCH="arm64"
    ;;
  x86_64-apple-darwin)
    FFMPEG_ARCH="x86_64"
    CLANG_ARCH="x86_64"
    ;;
  *)
    echo "Unsupported FFmpeg target: ${TARGET_TRIPLE}" >&2
    exit 1
    ;;
esac

if [ "$(uname -s)" != "Darwin" ]; then
  echo "The LocalScribe FFmpeg sidecar currently supports macOS only." >&2
  exit 1
fi

mkdir -p "${DOWNLOAD_DIR}" "$(dirname -- "${SOURCE_DIR}")" "${BUILD_DIR}" "$(dirname -- "${OUTPUT_PATH}")"

if [ ! -f "${ARCHIVE_PATH}" ]; then
  echo "Downloading FFmpeg ${FFMPEG_VERSION} from ffmpeg.org..."
  curl --fail --location --retry 3 --output "${ARCHIVE_PATH}.part" "${FFMPEG_URL}"
  mv "${ARCHIVE_PATH}.part" "${ARCHIVE_PATH}"
fi

ACTUAL_SHA256=$(shasum -a 256 "${ARCHIVE_PATH}" | awk '{print $1}')
if [ "${ACTUAL_SHA256}" != "${FFMPEG_SHA256}" ]; then
  echo "FFmpeg source checksum mismatch." >&2
  echo "Expected: ${FFMPEG_SHA256}" >&2
  echo "Actual:   ${ACTUAL_SHA256}" >&2
  exit 1
fi

if [ ! -x "${SOURCE_DIR}/configure" ]; then
  rm -rf "${SOURCE_DIR}"
  tar -xJf "${ARCHIVE_PATH}" -C "$(dirname -- "${SOURCE_DIR}")"
fi

SDK_PATH=$(xcrun --sdk macosx --show-sdk-path)
MIN_MACOS_VERSION="13.0"

cd "${BUILD_DIR}"
"${SOURCE_DIR}/configure" \
  --arch="${FFMPEG_ARCH}" \
  --target-os=darwin \
  --cc=clang \
  --sysroot="${SDK_PATH}" \
  --enable-cross-compile \
  --disable-gpl \
  --disable-nonfree \
  --disable-autodetect \
  --disable-network \
  --disable-doc \
  --disable-debug \
  --disable-ffplay \
  --disable-ffprobe \
  --disable-avdevice \
  --disable-shared \
  --enable-static \
  --enable-pic \
  --extra-cflags="-arch ${CLANG_ARCH} -mmacosx-version-min=${MIN_MACOS_VERSION}" \
  --extra-ldflags="-arch ${CLANG_ARCH} -mmacosx-version-min=${MIN_MACOS_VERSION}"

grep -F '#define CONFIG_GPL 0' config.h >/dev/null
grep -F '#define CONFIG_NONFREE 0' config.h >/dev/null
grep -F '#define CONFIG_VERSION3 0' config.h >/dev/null

JOBS=$(sysctl -n hw.logicalcpu 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)
make -j "${JOBS}" ffmpeg

cp "${BUILD_DIR}/ffmpeg" "${OUTPUT_PATH}"
chmod 755 "${OUTPUT_PATH}"

VERSION_OUTPUT=$("${OUTPUT_PATH}" -version 2>&1)
echo "${VERSION_OUTPUT}" | grep -F "ffmpeg version ${FFMPEG_VERSION}" >/dev/null
if echo "${VERSION_OUTPUT}" | grep -E -- '--enable-(gpl|nonfree)' >/dev/null; then
  echo "Refusing to package a GPL or nonfree FFmpeg build." >&2
  exit 1
fi

echo "Prepared LGPL-only FFmpeg sidecar: ${OUTPUT_PATH}"
echo "Verified source archive: ${ARCHIVE_PATH}"
