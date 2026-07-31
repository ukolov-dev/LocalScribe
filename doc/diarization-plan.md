# LocalScribe Speaker Diarization Plan

## Goal

Add optional local speaker diarization without bundling model weights into the app. Users can download speaker model packs separately, similar to ASR models.

## Model Packs

| Pack | Size | Status | Notes |
| --- | ---: | --- | --- |
| Speaker Lite | ~29 MB | Recommended | Silero VAD + WeSpeaker ResNet34-LM ONNX. Best first target for Rust/ONNX Runtime integration. |
| ECAPA-TDNN | ~86 MB | Better quality | Silero VAD + SpeechBrain ECAPA checkpoint. Needs conversion/runtime adapter before inference. |
| TitaNet Large | ~97 MB | Better quality | Silero VAD + NVIDIA TitaNet NeMo archive. Needs conversion/runtime adapter before inference. |
| Custom speaker ONNX | variable | Manual | User-selected local ONNX embedding model. |

## Storage

```text
~/Library/Application Support/LocalScribe/models/diarization/
```

Each pack stores its files under a pack-specific subdirectory:

```text
models/diarization/speaker-lite/
models/diarization/speaker-ecapa/
models/diarization/speaker-titanet/
```

Downloads use `.part` files and atomic rename after validation.

## Implementation Phases

1. Model manager
   - List/download/delete speaker model packs.
   - Track progress across multi-file packs.
   - Support custom ONNX speaker embedding model.

2. Inference backend
   - Add ONNX Runtime.
   - Run Silero VAD on prepared 16 kHz mono samples.
   - Run WeSpeaker embeddings over speech windows.
   - Cluster embeddings with cosine distance.

3. Transcript alignment
   - Assign speaker labels to Whisper segments by timestamp overlap.
   - Add optional `speaker` field to transcript segments.
   - Keep user-editable speaker labels.

4. UI and export
   - Show `Speaker 1`, `Speaker 2`, etc. in editor.
   - Allow renaming speakers.
   - Include labels in TXT/SRT/VTT exports.

5. Quality improvements
   - Expected speaker count: Auto/2/3/4.
   - Merge short speaker turns.
   - Add ECAPA/TitaNet adapters after ONNX path is stable.
