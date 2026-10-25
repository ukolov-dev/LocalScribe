# Local neural speaker diarization

The former pitch/spectrum heuristic and fixed two-cluster k-means are removed.

## Pipeline

1. First use automatically installs the `speaker-neural` pack (45,586,674 bytes). Revisions and SHA-256 hashes are pinned in `speaker_models.rs`; downloads use `.part` files and atomic rename. Cached files are verified before inference too. Offline use requires this one-time download.
2. FFmpeg converts input into 16 kHz mono PCM. The same prepared audio feeds both models and Whisper.
3. A private `--diarize` subprocess runs sherpa-onnx 2.0.0: Pyannote segmentation 3.0 → 3D-Speaker ERes2Net embeddings → cosine clustering with automatic cluster count (`num_clusters = -1`, threshold 0.9). No two-person cap or assumed participant count is supplied. Labels are numbered by first appearance within each recording.
4. Whisper computes token timestamps internally, even when display timestamps are disabled. Tokens are grouped into words without breaking UTF-8; each word is matched to the voice with greatest time overlap. Adjacent words of the same voice form a readable turn. Original sentence filtering happens before word alignment.
5. The final editor, TXT/Markdown export and meeting integration receive the same labeled segments. The existing meeting roster can still map anonymous labels to names using an independently supplied active-speaker timeline.

## Runtime and privacy

- Inference uses the CPU locally, with two ONNX threads. Python, a server, account tokens and cloud speech services are not needed.
- Models live in `~/Library/Application Support/com.localscribe.desktop/models/diarization/speaker-neural/`.
- The private speaker worker is cancelled by killing and reaping that exact child. It also exits if its parent disappears. Its temporary JSON is owner-only and removed after success, failure or cancellation.
- The UI exposes only the active neural pack. Older downloaded speaker packs/custom paths remain untouched for compatibility and are not used for inference.

## Limits and checks

A model can merge similar voices or split one voice after microphone/noise changes. Very short interjections and overlapping speech remain difficult. A single-channel transcription cannot reconstruct two independently intelligible simultaneous utterances. Speech farther than 500 ms from any detected voice is marked `Неизвестный спикер`, rather than inventing a confident label.

Run `cargo test --manifest-path src-tauri/Cargo.toml` for alignment, four-label stability, cancellation and export regressions. The end-to-end smoke script below downloads the official public four-speaker test and pinned models, then checks the real neural output with automatic count:

```bash
python3 scripts/test-neural-diarization.py
```

The test is a functional smoke test, not a measured diarization error rate on meetings.

Upstream: [sherpa-onnx models and sample](https://k2-fsa.github.io/sherpa/onnx/speaker-diarization/models.html), [3D-Speaker](https://github.com/modelscope/3D-Speaker), [Pyannote segmentation](https://huggingface.co/pyannote/segmentation-3.0).

## Local verification (2026-10-24)

- 58 Rust tests and 5 Swift tests passed; legacy frontend tests/build passed.
- Public 56.861 s four-speaker WAV: automatic count = 4; all nine reference identity checks passed, including returning speakers. Neural pass took 6.23 s on this Mac (not a cross-device performance claim).
- Full private-worker → Whisper CPU → token alignment → TXT/Markdown flow completed in 13.08 s. All four labels survived export with display timestamps disabled. Unmatched speech/hallucinations in silent gaps were explicitly marked unknown.
- First-use download cancellation and active neural-worker cancellation passed; no test workers remained afterward.
- This is not a DER benchmark or a guarantee on noisy multi-person meetings.
