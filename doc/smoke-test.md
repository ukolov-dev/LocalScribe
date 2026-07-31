# Smoke Test

The smoke test validates:

- bundled ffmpeg sidecar can read MP3 input;
- audio is converted to `16 kHz mono PCM WAV`;
- `large-v3-turbo-q5_0` loads through `whisper-rs`;
- `TXT` and `MD` exports are generated.

## Commands

```bash
npm run prepare-sidecars

mkdir -p /tmp/localscribe-smoke
curl -L -o /tmp/localscribe-smoke/jfk.wav \
  https://github.com/ggerganov/whisper.cpp/raw/master/samples/jfk.wav

src-tauri/binaries/ffmpeg-aarch64-apple-darwin \
  -y -hide_banner -loglevel error \
  -i /tmp/localscribe-smoke/jfk.wav \
  /tmp/localscribe-smoke/jfk.mp3

curl -L -C - \
  -o /tmp/localscribe-smoke/ggml-large-v3-turbo-q5_0.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin

shasum -a 256 /tmp/localscribe-smoke/ggml-large-v3-turbo-q5_0.bin

cd src-tauri
cargo run --example smoke_transcribe -- \
  /tmp/localscribe-smoke/jfk.mp3 \
  /tmp/localscribe-smoke/ggml-large-v3-turbo-q5_0.bin \
  /tmp/localscribe-smoke/out \
  en
```

Expected SHA256:

```text
394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2
```

Expected transcript:

```text
And so, my fellow Americans, ask not what your country can do for you, ask what you can do for your country.
```

Outputs:

```text
/tmp/localscribe-smoke/out/transcript.txt
/tmp/localscribe-smoke/out/transcript.md
```

Note: `smoke_transcribe` runs CPU by default so it can work in headless/sandboxed environments. Set `LOCALSCRIBE_SMOKE_USE_GPU=1` to exercise Metal. In this local environment, the Metal smoke run failed during `ggml_metal_buffer_init`, while CPU smoke passed.
