# Media fixtures

The files in `media/` are half-second synthetic test signals used to verify the
LGPL-only FFmpeg sidecar against every media extension accepted by LocalScribe:
AAC (ADTS), WAV, MP3, M4A, MP4, MOV, and WebM.

They contain only a generated 440 Hz sine wave and, for video containers, a
solid-color 160×90 frame. No third-party audio, video, or personal data is
included. The fixtures were generated with FFmpeg 8.1.1 from `lavfi` sources.

The fixture encoder is not redistributed with LocalScribe. Tests exercise only
the repository-built FFmpeg 8.1.2 sidecar as the decoder.
