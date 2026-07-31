import { invoke } from "@tauri-apps/api/core";
import type {
  ExportFormat,
  ModelInfo,
  SpeakerModelInfo,
  TranscribeOptions,
  TranscriptSegment,
  TranscriptionResult
} from "../types/localscribe";

export async function listModels(): Promise<ModelInfo[]> {
  return invoke<ModelInfo[]>("list_models");
}

export async function downloadModel(modelId: string): Promise<ModelInfo> {
  return invoke<ModelInfo>("download_model", { modelId });
}

export async function deleteModel(modelId: string): Promise<ModelInfo[]> {
  return invoke<ModelInfo[]>("delete_model", { modelId });
}

export async function selectCustomModel(path: string): Promise<ModelInfo> {
  return invoke<ModelInfo>("select_custom_model", { path });
}

export async function listSpeakerModels(): Promise<SpeakerModelInfo[]> {
  return invoke<SpeakerModelInfo[]>("list_speaker_models");
}

export async function downloadSpeakerModel(modelId: string): Promise<SpeakerModelInfo> {
  return invoke<SpeakerModelInfo>("download_speaker_model", { modelId });
}

export async function deleteSpeakerModel(modelId: string): Promise<SpeakerModelInfo[]> {
  return invoke<SpeakerModelInfo[]>("delete_speaker_model", { modelId });
}

export async function selectCustomSpeakerModel(path: string): Promise<SpeakerModelInfo> {
  return invoke<SpeakerModelInfo>("select_custom_speaker_model", { path });
}

export async function transcribeFile(
  options: TranscribeOptions
): Promise<TranscriptionResult> {
  return invoke<TranscriptionResult>("transcribe_file", { options });
}

export async function cancelTranscription(jobId: string): Promise<boolean> {
  return invoke<boolean>("cancel_transcription", { jobId });
}

export async function exportTranscript(params: {
  format: ExportFormat;
  outputPath: string;
  text: string;
  segments: TranscriptSegment[];
}): Promise<string> {
  return invoke<string>("export_transcript", {
    request: params
  });
}

export function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error === "object") {
    const maybeError = error as { message?: string; code?: string };
    if (maybeError.message && maybeError.code) {
      return `${maybeError.message} (${maybeError.code})`;
    }
    if (maybeError.message) {
      return maybeError.message;
    }
  }
  return "Unexpected application error.";
}
