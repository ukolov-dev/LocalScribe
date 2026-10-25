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
    // Accepted for older clients; transcription always assigns speakers.
    pub speaker_mode: Option<String>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationOptions {
    pub language: Option<String>,
    pub decoding_preset: Option<String>,
    pub speaker_mode: Option<String>,
    pub timestamps: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MeetingParticipant {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant_id: Option<String>,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerTimelineEvent {
    pub at_ms: u64,
    #[serde(default)]
    pub speakers: Vec<MeetingParticipant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerAssignment {
    pub speaker: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant_id: Option<String>,
    pub display_name: String,
    pub confidence: f32,
    pub state: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationRequest {
    pub schema_version: u32,
    pub id: String,
    pub source: String,
    pub meeting_id: Option<String>,
    #[serde(default)]
    pub meeting_title: Option<String>,
    #[serde(default)]
    pub participants: Vec<MeetingParticipant>,
    #[serde(default)]
    pub speaker_timeline: Vec<SpeakerTimelineEvent>,
    pub media_path: String,
    pub original_file_name: String,
    pub requested_at: String,
    #[serde(default)]
    pub options: IntegrationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationCompletion {
    pub job_id: String,
    pub transcript_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummary {
    pub text: String,
    pub model: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingRecord {
    pub schema_version: u32,
    pub id: String,
    pub source: String,
    pub meeting_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub media_path: String,
    pub transcript_path: String,
    pub status: String,
    #[serde(default)]
    pub participants: Vec<MeetingParticipant>,
    #[serde(default)]
    pub speaker_assignments: Vec<SpeakerAssignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<MeetingSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingDetail {
    #[serde(flatten)]
    pub meeting: MeetingRecord,
    pub transcript: String,
    #[serde(default)]
    pub segments: Vec<TranscriptSegment>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_speaker_model_option_does_not_break_transcription_requests() {
        let options: TranscribeOptions = serde_json::from_str(
            r#"{
            "filePath": "/tmp/input.wav", "language": "en", "timestamps": true,
            "speakerMode": "auto", "speakerModelId": "speaker-lite"
        }"#,
        )
        .unwrap();
        assert_eq!(options.speaker_mode.as_deref(), Some("auto"));
        assert!(!serde_json::to_string(&options)
            .unwrap()
            .contains("speakerModelId"));
    }
}
