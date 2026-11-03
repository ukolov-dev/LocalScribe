# Media fixtures

The files in `media/` are half-second synthetic test signals used to verify the
LGPL-only FFmpeg sidecar against every media extension accepted by LocalScribe:
AAC (ADTS), WAV, MP3, M4A, MP4, MOV, WebM, and MKV.

They contain only a generated 440 Hz sine wave and, for video containers, a
solid-color 160×90 frame. No third-party audio, video, or personal data is
included. The fixtures were generated with FFmpeg 8.1.1 from `lavfi` sources.
The MKV fixture preserves the MP4 fixture's H.264 video and AAC audio by remuxing
with the bundled FFmpeg 8.1.2, without re-encoding:

```sh
src-tauri/binaries/ffmpeg-aarch64-apple-darwin -i src-tauri/tests/fixtures/media/sample.mp4 \
  -map 0 -c copy src-tauri/tests/fixtures/media/sample.mkv
```

The fixture encoder is not redistributed with LocalScribe. Tests exercise only
the repository-built FFmpeg 8.1.2 sidecar as the decoder.
