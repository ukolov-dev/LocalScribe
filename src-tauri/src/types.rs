use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub file_name: Option<String>,
    pub size_label: String,
    pub size_bytes: Option<u64>,
    pub recommended: bool,
    pub quality_label: String,
    pub installed: bool,
    pub path: Option<String>,
    pub custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadCompleted {
    pub model: ModelInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerModelFileInfo {
    pub id: String,
    pub file_name: String,
    pub path: Option<String>,
    pub installed: bool,
    pub size_label: String,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerModelInfo {
    pub id: String,
    pub name: String,
    pub size_label: String,
    pub size_bytes: Option<u64>,
    pub recommended: bool,
    pub quality_label: String,
    pub runtime_label: String,
    pub installed: bool,
    pub custom: bool,
    pub files: Vec<SpeakerModelFileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerModelDownloadCompleted {
    pub model: SpeakerModelInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectCustomModelRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeOptions {
    pub job_id: Option<String>,
    pub file_path: String,
    pub model_id: Option<String>,
    pub model_path: Option<String>,
    pub engine: Option<String>,
    pub speaker_mode: Option<String>,
    pub speaker_model_id: Option<String>,
    pub language: String,
    pub decoding_preset: Option<String>,
    pub initial_prompt: Option<String>,
    pub timestamps: bool,
    pub threads: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionProgress {
    pub job_id: String,
    pub phase: String,
    pub progress: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkPhase {
    pub phase: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionBenchmark {
    pub total_ms: u64,
    pub audio_duration_ms: u64,
    pub phases: Vec<BenchmarkPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub index: u32,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionSegmentEvent {
    pub job_id: String,
    pub segment: TranscriptSegment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub job_id: String,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark: Option<TranscriptionBenchmark>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionError {
    pub job_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Txt,
    Md,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub output_path: String,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
}
