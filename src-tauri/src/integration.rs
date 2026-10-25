use crate::{
    errors::{display_path, invalid, AppError, AppResult},
    export,
    types::{
        ExportFormat, IntegrationCompletion, IntegrationRequest, MeetingDetail, MeetingParticipant,
        MeetingRecord, MeetingSummary, SpeakerAssignment, SpeakerTimelineEvent, TranscriptSegment,
        TranscriptionResult,
    },
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tauri::AppHandle;
use uuid::Uuid;

const SCHEMA_VERSION: u32 = 1;
const SUPPORTED_EXTENSIONS: &[&str] = &["aac", "mp3", "wav", "m4a", "mp4", "mov", "webm"];

pub fn initialize(app: &AppHandle) -> AppResult<()> {
    let dir = integration_dir(app)?;
    initialize_dir(&dir)?;
    backfill_legacy_meetings(&dir)
}

pub(crate) fn root_dir(app: &AppHandle) -> AppResult<PathBuf> {
    integration_dir(app)
}

pub fn claim_next(app: &AppHandle) -> AppResult<Option<IntegrationRequest>> {
    claim_next_in_dir(&integration_dir(app)?)
}

fn claim_next_in_dir(dir: &Path) -> AppResult<Option<IntegrationRequest>> {
    initialize_dir(dir)?;

    if let Some(request) = recover_processing_request(dir)? {
        return Ok(Some(request));
    }

    let inbox = dir.join("inbox");
    let processing = dir.join("processing");

    let entries = sorted_json_entries(&inbox)?;

    for entry in entries {
        let path = entry.path();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) => {
                quarantine_invalid_request_in_dir(dir, &path, &error.to_string())?;
                continue;
            }
        };
        let request: IntegrationRequest = match serde_json::from_str(&raw) {
            Ok(request) => request,
            Err(error) => {
                quarantine_invalid_request_in_dir(dir, &path, &error.to_string())?;
                continue;
            }
        };

        if let Err(error) = validate_request(&request) {
            quarantine_invalid_request_in_dir(dir, &path, &error.to_string())?;
            continue;
        }

        let processing_path = processing.join(format!("{}.json", request.id));
        match fs::rename(&path, &processing_path) {
            Ok(()) => return Ok(Some(request)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
    }

    Ok(None)
}

fn recover_processing_request(dir: &Path) -> AppResult<Option<IntegrationRequest>> {
    for entry in sorted_json_entries(&dir.join("processing"))? {
        let path = entry.path();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) => {
                quarantine_invalid_request_in_dir(dir, &path, &error.to_string())?;
                continue;
            }
        };
        let request: IntegrationRequest = match serde_json::from_str(&raw) {
            Ok(request) => request,
            Err(error) => {
                quarantine_invalid_request_in_dir(dir, &path, &error.to_string())?;
                continue;
            }
        };

        if let Err(error) = validate_request_metadata(&request) {
            quarantine_invalid_request_in_dir(dir, &path, &error.to_string())?;
            continue;
        }
        if path.file_name() != Some(format!("{}.json", request.id).as_ref()) {
            quarantine_invalid_request_in_dir(
                dir,
                &path,
                "The processing file name does not match the integration job id.",
            )?;
            continue;
        }

        let receipt_name = format!("{}.json", request.id);
        if dir.join("completed").join(&receipt_name).is_file()
            || dir.join("failed").join(&receipt_name).is_file()
        {
            fs::remove_file(path)?;
            continue;
        }

        if let Err(error) = validate_media_path(&request.media_path) {
            quarantine_invalid_request_in_dir(dir, &path, &error.to_string())?;
            continue;
        }

        return Ok(Some(request));
    }

    Ok(None)
}

fn sorted_json_entries(dir: &Path) -> AppResult<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}

pub fn complete(
    app: &AppHandle,
    job_id: &str,
    result: &TranscriptionResult,
) -> AppResult<IntegrationCompletion> {
    complete_in_dir(&integration_dir(app)?, job_id, result)
}

fn complete_in_dir(
    dir: &Path,
    job_id: &str,
    result: &TranscriptionResult,
) -> AppResult<IntegrationCompletion> {
    initialize_dir(dir)?;
    validate_job_id(job_id)?;
    if result.job_id != job_id {
        return Err(invalid(
            "The transcription result does not match the integration job.",
        ));
    }
    let processing_path = processing_path(dir, job_id);
    let request: IntegrationRequest = serde_json::from_str(&fs::read_to_string(&processing_path)?)?;
    validate_request(&request)?;
    if request.id != job_id {
        return Err(invalid(
            "The claimed integration request does not match the integration job.",
        ));
    }

    let mut attributed_result = result.clone();
    let speaker_assignments = assign_participant_names(
        &mut attributed_result.segments,
        &request.participants,
        &request.speaker_timeline,
    );
    let transcript_path = transcript_path(&request.media_path, job_id)?;
    let transcript = export::format_transcript(
        ExportFormat::Md,
        &attributed_result.text,
        &attributed_result.segments,
    );
    atomic_write(&transcript_path, transcript.as_bytes())?;
    let segments_path = dir.join("segments").join(format!("{job_id}.json"));
    atomic_write(
        &segments_path,
        &serde_json::to_vec_pretty(&attributed_result.segments)?,
    )?;

    let meeting = MeetingRecord {
        schema_version: SCHEMA_VERSION,
        id: job_id.to_string(),
        source: request.source.clone(),
        meeting_id: request.meeting_id.clone(),
        title: meeting_title(&request),
        created_at: request.requested_at.clone(),
        media_path: request.media_path.clone(),
        transcript_path: display_path(&transcript_path),
        status: "completed".to_string(),
        participants: request.participants.clone(),
        speaker_assignments,
        segments_path: Some(display_path(&segments_path)),
        summary: None,
    };
    write_meeting_in_dir(dir, &meeting)?;

    let completed = dir.join("completed");
    let receipt_path = completed.join(format!("{job_id}.json"));
    let receipt = serde_json::to_vec_pretty(&json!({
        "schemaVersion": SCHEMA_VERSION,
        "jobId": job_id,
        "source": request.source,
        "meetingId": request.meeting_id,
        "meetingTitle": request.meeting_title,
        "createdAt": request.requested_at,
        "mediaPath": request.media_path,
        "transcriptPath": display_path(&transcript_path),
        "segmentsPath": display_path(&segments_path),
        "participants": request.participants,
        "status": "completed"
    }))?;
    atomic_write(&receipt_path, &receipt)?;
    fs::remove_file(processing_path)?;

    Ok(IntegrationCompletion {
        job_id: job_id.to_string(),
        transcript_path: display_path(&transcript_path),
    })
}

pub fn fail(app: &AppHandle, job_id: &str, message: &str) -> AppResult<()> {
    fail_in_dir(&integration_dir(app)?, job_id, message)
}

fn fail_in_dir(dir: &Path, job_id: &str, message: &str) -> AppResult<()> {
    initialize_dir(dir)?;
    validate_job_id(job_id)?;
    let processing_path = processing_path(dir, job_id);
    let failed = dir.join("failed");
    let receipt_path = failed.join(format!("{job_id}.json"));
    let receipt = serde_json::to_vec_pretty(&json!({
        "schemaVersion": SCHEMA_VERSION,
        "jobId": job_id,
        "status": "failed",
        "message": message
    }))?;
    atomic_write(&receipt_path, &receipt)?;
    if processing_path.exists() {
        fs::remove_file(processing_path)?;
    }
    Ok(())
}

fn integration_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = crate::storage::data_dir(app)?.join("integration");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn initialize_dir(dir: &Path) -> AppResult<()> {
    for child in [
        "inbox",
        "processing",
        "completed",
        "failed",
        "meetings",
        "segments",
    ] {
        fs::create_dir_all(dir.join(child))?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyCompletion {
    job_id: String,
    meeting_id: Option<String>,
    media_path: String,
    transcript_path: String,
    status: String,
}

fn backfill_legacy_meetings(dir: &Path) -> AppResult<()> {
    for entry in sorted_json_entries(&dir.join("completed"))? {
        let receipt: LegacyCompletion =
            match serde_json::from_str(&fs::read_to_string(entry.path())?) {
                Ok(receipt) => receipt,
                Err(_) => continue,
            };
        if receipt.status != "completed" || validate_job_id(&receipt.job_id).is_err() {
            continue;
        }
        let destination = meeting_path(dir, &receipt.job_id);
        if destination.exists() || !Path::new(&receipt.transcript_path).is_file() {
            continue;
        }
        let title = Path::new(&receipt.media_path)
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("eXpress meeting")
            .to_string();
        let created_at = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis().to_string())
            .unwrap_or_default();
        write_meeting_in_dir(
            dir,
            &MeetingRecord {
                schema_version: SCHEMA_VERSION,
                id: receipt.job_id,
                source: "express".to_string(),
                meeting_id: receipt.meeting_id,
                title,
                created_at,
                media_path: receipt.media_path,
                transcript_path: receipt.transcript_path,
                status: "completed".to_string(),
                participants: Vec::new(),
                speaker_assignments: Vec::new(),
                segments_path: None,
                summary: None,
            },
        )?;
    }
    Ok(())
}

pub(crate) fn list_meetings_in_dir(dir: &Path) -> AppResult<Vec<MeetingRecord>> {
    initialize_dir(dir)?;
    let mut meetings = Vec::new();
    for entry in sorted_json_entries(&dir.join("meetings"))? {
        let record: MeetingRecord = serde_json::from_str(&fs::read_to_string(entry.path())?)?;
        validate_job_id(&record.id)?;
        meetings.push(record);
    }
    meetings.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(meetings)
}

pub(crate) fn meeting_in_dir(dir: &Path, id: &str) -> AppResult<MeetingDetail> {
    validate_job_id(id)?;
    let record: MeetingRecord = serde_json::from_str(&fs::read_to_string(meeting_path(dir, id))?)?;
    if record.id != id {
        return Err(invalid(
            "The meeting record id does not match its file name.",
        ));
    }
    let transcript = fs::read_to_string(&record.transcript_path)?;
    let segments = record
        .segments_path
        .as_deref()
        .filter(|path| Path::new(path).is_file())
        .map(fs::read_to_string)
        .transpose()?
        .map(|raw| serde_json::from_str(&raw))
        .transpose()?
        .unwrap_or_default();
    Ok(MeetingDetail {
        meeting: record,
        transcript,
        segments,
    })
}

pub(crate) fn save_summary_in_dir(
    dir: &Path,
    id: &str,
    summary: MeetingSummary,
) -> AppResult<MeetingDetail> {
    validate_job_id(id)?;
    validate_summary(&summary)?;
    let path = meeting_path(dir, id);
    let mut record: MeetingRecord = serde_json::from_str(&fs::read_to_string(&path)?)?;
    if record.id != id {
        return Err(invalid(
            "The meeting record id does not match its file name.",
        ));
    }
    record.summary = Some(summary);
    write_meeting_in_dir(dir, &record)?;
    meeting_in_dir(dir, id)
}

fn write_meeting_in_dir(dir: &Path, meeting: &MeetingRecord) -> AppResult<()> {
    validate_job_id(&meeting.id)?;
    let contents = serde_json::to_vec_pretty(meeting)?;
    atomic_write(&meeting_path(dir, &meeting.id), &contents)
}

fn meeting_path(dir: &Path, id: &str) -> PathBuf {
    dir.join("meetings").join(format!("{id}.json"))
}

fn meeting_title(request: &IntegrationRequest) -> String {
    request
        .meeting_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            Path::new(&request.original_file_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "eXpress meeting".to_string())
}

fn participant_key(participant: &MeetingParticipant) -> String {
    participant
        .participant_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("id:{value}"))
        .unwrap_or_else(|| format!("name:{}", participant.display_name.to_lowercase()))
}

fn interval_overlap(start_a: u64, end_a: u64, start_b: u64, end_b: u64) -> u64 {
    end_a.min(end_b).saturating_sub(start_a.max(start_b))
}

#[derive(Clone)]
struct AttributionCandidate {
    speaker: String,
    participant: MeetingParticipant,
    score_ms: u64,
    confidence: f32,
    segment_hits: usize,
}

fn assign_participant_names(
    segments: &mut [TranscriptSegment],
    participants: &[MeetingParticipant],
    speaker_timeline: &[SpeakerTimelineEvent],
) -> Vec<SpeakerAssignment> {
    if segments.is_empty() || speaker_timeline.is_empty() {
        return Vec::new();
    }

    let recording_end = segments
        .iter()
        .map(|segment| segment.end_ms)
        .max()
        .unwrap_or(0);
    let mut timeline = speaker_timeline.to_vec();
    timeline.sort_by_key(|event| event.at_ms);

    let mut known_participants = HashMap::<String, MeetingParticipant>::new();
    for participant in participants {
        known_participants.insert(participant_key(participant), participant.clone());
    }
    for event in &timeline {
        for participant in &event.speakers {
            known_participants
                .entry(participant_key(participant))
                .or_insert_with(|| participant.clone());
        }
    }

    let mut scores = HashMap::<(String, String), u64>::new();
    let mut hits = HashSet::<(String, String, u32)>::new();
    for segment in segments.iter() {
        let Some(speaker) = segment
            .speaker
            .as_deref()
            .map(str::trim)
            .filter(|speaker| !speaker.is_empty())
        else {
            continue;
        };
        for (index, event) in timeline.iter().enumerate() {
            if event.speakers.len() != 1 {
                continue;
            }
            let interval_end = timeline
                .get(index + 1)
                .map(|next| next.at_ms)
                .unwrap_or(recording_end)
                .min(recording_end);
            let overlap = interval_overlap(
                segment.start_ms,
                segment.end_ms,
                event.at_ms.min(recording_end),
                interval_end,
            );
            if overlap == 0 {
                continue;
            }
            let key = participant_key(&event.speakers[0]);
            *scores
                .entry((speaker.to_string(), key.clone()))
                .or_default() += overlap;
            hits.insert((speaker.to_string(), key, segment.index));
        }
    }

    let speakers = segments
        .iter()
        .filter_map(|segment| segment.speaker.as_deref())
        .map(str::trim)
        .filter(|speaker| !speaker.is_empty())
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();
    let mut candidates = Vec::new();
    for speaker in speakers {
        let mut ranked = scores
            .iter()
            .filter(|((candidate_speaker, _), _)| candidate_speaker == &speaker)
            .map(|((_, key), score)| (key.clone(), *score))
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| right.1.cmp(&left.1));
        let Some((best_key, best_score)) = ranked.first().cloned() else {
            continue;
        };
        let second_score = ranked.get(1).map(|candidate| candidate.1).unwrap_or(0);
        let total_score = ranked.iter().map(|candidate| candidate.1).sum::<u64>();
        let confidence = best_score as f32 / total_score.max(1) as f32;
        let segment_hits = hits
            .iter()
            .filter(|(candidate_speaker, key, _)| candidate_speaker == &speaker && key == &best_key)
            .count();
        let enough_evidence = best_score >= 1_500 && (segment_hits >= 2 || best_score >= 4_000);
        let clear_winner = confidence >= 0.67
            && (second_score == 0
                || best_score.saturating_mul(2) >= second_score.saturating_mul(3));
        if !enough_evidence || !clear_winner {
            continue;
        }
        let Some(participant) = known_participants.get(&best_key).cloned() else {
            continue;
        };
        candidates.push(AttributionCandidate {
            speaker,
            participant,
            score_ms: best_score,
            confidence,
            segment_hits,
        });
    }

    candidates.sort_by(|left, right| {
        right
            .confidence
            .total_cmp(&left.confidence)
            .then_with(|| right.score_ms.cmp(&left.score_ms))
    });
    let mut used_participants = HashSet::new();
    let mut accepted = HashMap::<String, AttributionCandidate>::new();
    for candidate in candidates {
        let key = participant_key(&candidate.participant);
        if used_participants.insert(key) {
            accepted.insert(candidate.speaker.clone(), candidate);
        }
    }

    for segment in segments.iter_mut() {
        let Some(speaker) = segment.speaker.as_deref() else {
            continue;
        };
        if let Some(candidate) = accepted.get(speaker) {
            let confirmed = candidate.confidence >= 0.80
                && candidate.score_ms >= 6_000
                && candidate.segment_hits >= 3;
            if confirmed {
                segment.speaker = Some(candidate.participant.display_name.clone());
            }
        }
    }

    let mut assignments = accepted
        .into_values()
        .map(|candidate| {
            let confirmed = candidate.confidence >= 0.80
                && candidate.score_ms >= 6_000
                && candidate.segment_hits >= 3;
            SpeakerAssignment {
                speaker: candidate.speaker,
                participant_id: candidate.participant.participant_id,
                display_name: candidate.participant.display_name,
                confidence: candidate.confidence,
                state: if confirmed { "confirmed" } else { "probable" }.to_string(),
                source: "express-active-speaker-overlap".to_string(),
            }
        })
        .collect::<Vec<_>>();
    assignments.sort_by(|left, right| left.speaker.cmp(&right.speaker));
    assignments
}

fn validate_summary(summary: &MeetingSummary) -> AppResult<()> {
    let valid_text = !summary.text.trim().is_empty() && summary.text.len() <= 512 * 1024;
    let valid_model = !summary.model.trim().is_empty()
        && summary.model.len() <= 256
        && !summary.model.chars().any(char::is_control);
    let valid_date = !summary.generated_at.trim().is_empty()
        && summary.generated_at.len() <= 128
        && !summary.generated_at.chars().any(char::is_control);
    if valid_text && valid_model && valid_date {
        Ok(())
    } else {
        Err(invalid("Invalid meeting summary."))
    }
}

fn processing_path(dir: &Path, job_id: &str) -> PathBuf {
    dir.join("processing").join(format!("{job_id}.json"))
}

fn validate_request(request: &IntegrationRequest) -> AppResult<()> {
    validate_request_metadata(request)?;
    validate_media_path(&request.media_path)
}

fn validate_request_metadata(request: &IntegrationRequest) -> AppResult<()> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(invalid(format!(
            "Unsupported integration schema version: {}.",
            request.schema_version
        )));
    }
    if request.source != "express" {
        return Err(invalid("Only eXpress integration requests are accepted."));
    }
    if request
        .meeting_title
        .as_deref()
        .is_some_and(|title| title.len() > 512 || title.chars().any(char::is_control))
    {
        return Err(invalid("Invalid eXpress meeting title."));
    }
    if request.participants.len() > 100 || request.speaker_timeline.len() > 50_000 {
        return Err(invalid("Invalid eXpress speaker metadata."));
    }
    for participant in &request.participants {
        validate_participant(participant)?;
    }
    let mut previous_at_ms = 0;
    for (index, event) in request.speaker_timeline.iter().enumerate() {
        if event.at_ms > 24 * 60 * 60 * 1000
            || (index > 0 && event.at_ms < previous_at_ms)
            || event.speakers.len() > 16
        {
            return Err(invalid("Invalid eXpress speaker timeline."));
        }
        previous_at_ms = event.at_ms;
        for participant in &event.speakers {
            validate_participant(participant)?;
        }
    }
    validate_job_id(&request.id)
}

fn validate_participant(participant: &MeetingParticipant) -> AppResult<()> {
    let valid_name = !participant.display_name.trim().is_empty()
        && participant.display_name.len() <= 256
        && !participant.display_name.chars().any(char::is_control);
    let valid_id = participant.participant_id.as_deref().map_or(true, |id| {
        id.len() <= 160 && !id.chars().any(char::is_control)
    });
    if valid_name && valid_id {
        Ok(())
    } else {
        Err(invalid("Invalid eXpress participant metadata."))
    }
}

fn validate_media_path(path: &str) -> AppResult<()> {
    let media_path = Path::new(path);
    if !media_path.is_file() {
        return Err(invalid("The eXpress recording file does not exist."));
    }
    let extension = media_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or(AppError::UnsupportedFormat)?;
    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(AppError::UnsupportedFormat);
    }
    Ok(())
}

fn validate_job_id(job_id: &str) -> AppResult<()> {
    let valid = !job_id.is_empty()
        && job_id.len() <= 160
        && job_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(invalid("Invalid integration job id."))
    }
}

fn transcript_path(media_path: &str, job_id: &str) -> AppResult<PathBuf> {
    validate_job_id(job_id)?;
    let media_path = Path::new(media_path);
    let parent = media_path
        .parent()
        .ok_or_else(|| invalid("The recording path has no parent directory."))?;
    Ok(parent.join(format!("{job_id}.localscribe.md")))
}

fn atomic_write(path: &Path, content: &[u8]) -> AppResult<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid("The output file name is invalid."))?;
    let temp_path = path.with_file_name(format!(".{file_name}.{}.part", Uuid::new_v4()));
    fs::write(&temp_path, content)?;
    match fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(temp_path);
            Err(error.into())
        }
    }
}

fn quarantine_invalid_request_in_dir(dir: &Path, path: &Path, message: &str) -> AppResult<()> {
    let failed = dir.join("failed");
    fs::create_dir_all(&failed)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("invalid-request.json");
    let destination = failed.join(file_name);
    let receipt = serde_json::to_vec_pretty(&json!({
        "schemaVersion": SCHEMA_VERSION,
        "status": "rejected",
        "message": message
    }))?;
    atomic_write(&destination, &receipt)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TranscriptSegment, TranscriptionResult};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("localscribe-integration-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn request(media_path: &Path, id: &str) -> IntegrationRequest {
        IntegrationRequest {
            schema_version: SCHEMA_VERSION,
            id: id.to_string(),
            source: "express".to_string(),
            meeting_id: Some("meeting-1".to_string()),
            meeting_title: Some("Product sync".to_string()),
            participants: Vec::new(),
            speaker_timeline: Vec::new(),
            media_path: display_path(media_path),
            original_file_name: "meeting.m4a".to_string(),
            requested_at: "2026-08-27T12:00:00Z".to_string(),
            options: Default::default(),
        }
    }

    fn write_request(path: &Path, request: &IntegrationRequest) {
        fs::write(path, serde_json::to_vec_pretty(request).unwrap()).unwrap();
    }

    #[test]
    fn accepts_safe_job_ids() {
        assert!(validate_job_id("express-123e4567-e89b-12d3-a456-426614174000_1").is_ok());
        assert!(validate_job_id("../escape").is_err());
        assert!(validate_job_id("").is_err());
    }

    #[test]
    fn places_transcript_next_to_recording() {
        let path = transcript_path("/tmp/Call recording [123].mp4", "express-123").unwrap();
        assert_eq!(path, PathBuf::from("/tmp/express-123.localscribe.md"));
    }

    #[test]
    fn markdown_export_keeps_segments() {
        let segments = vec![TranscriptSegment {
            index: 0,
            start_ms: 1_000,
            end_ms: 2_000,
            text: "Тест".to_string(),
            speaker: None,
        }];
        let markdown = export::format_transcript(ExportFormat::Md, "Тест", &segments);
        assert!(markdown.contains("## 00:01"));
        assert!(markdown.contains("Тест"));
    }

    #[test]
    fn initializes_the_full_queue_contract() {
        let dir = TestDir::new();
        initialize_dir(&dir.0).unwrap();

        for child in [
            "inbox",
            "processing",
            "completed",
            "failed",
            "meetings",
            "segments",
        ] {
            assert!(dir.0.join(child).is_dir());
        }
    }

    #[test]
    fn maps_diarized_speakers_to_express_participants_by_timeline() {
        let anna = MeetingParticipant {
            participant_id: Some("publisher-anna".to_string()),
            display_name: "Анна".to_string(),
        };
        let boris = MeetingParticipant {
            participant_id: Some("publisher-boris".to_string()),
            display_name: "Борис".to_string(),
        };
        let mut segments = (0..6)
            .map(|index| TranscriptSegment {
                index,
                start_ms: index as u64 * 3_000,
                end_ms: (index as u64 + 1) * 3_000,
                text: format!("segment {index}"),
                speaker: Some(
                    if index % 2 == 0 {
                        "Speaker 1"
                    } else {
                        "Speaker 2"
                    }
                    .to_string(),
                ),
            })
            .collect::<Vec<_>>();
        let timeline = (0..6)
            .map(|index| SpeakerTimelineEvent {
                at_ms: index as u64 * 3_000,
                speakers: vec![if index % 2 == 0 {
                    anna.clone()
                } else {
                    boris.clone()
                }],
            })
            .collect::<Vec<_>>();

        let assignments =
            assign_participant_names(&mut segments, &[anna.clone(), boris.clone()], &timeline);

        assert_eq!(segments[0].speaker.as_deref(), Some("Анна"));
        assert_eq!(segments[1].speaker.as_deref(), Some("Борис"));
        assert_eq!(assignments.len(), 2);
        assert!(assignments
            .iter()
            .all(|assignment| assignment.state == "confirmed"));
    }

    #[test]
    fn does_not_guess_names_from_roster_without_timeline() {
        let mut segments = vec![TranscriptSegment {
            index: 0,
            start_ms: 0,
            end_ms: 10_000,
            text: "Тест".to_string(),
            speaker: Some("Speaker 1".to_string()),
        }];
        let assignments = assign_participant_names(
            &mut segments,
            &[MeetingParticipant {
                participant_id: Some("publisher-anna".to_string()),
                display_name: "Анна".to_string(),
            }],
            &[],
        );

        assert!(assignments.is_empty());
        assert_eq!(segments[0].speaker.as_deref(), Some("Speaker 1"));
    }

    #[test]
    fn backfills_completed_receipts_into_the_meeting_library() {
        let dir = TestDir::new();
        initialize_dir(&dir.0).unwrap();
        let media_path = dir.0.join("legacy.webm");
        let transcript_path = dir.0.join("legacy.localscribe.md");
        fs::write(&media_path, b"audio").unwrap();
        fs::write(&transcript_path, "Legacy transcript").unwrap();
        fs::write(
            dir.0.join("completed/express-legacy.json"),
            serde_json::to_vec_pretty(&json!({
                "schemaVersion": 1,
                "jobId": "express-legacy",
                "meetingId": null,
                "mediaPath": display_path(&media_path),
                "transcriptPath": display_path(&transcript_path),
                "status": "completed"
            }))
            .unwrap(),
        )
        .unwrap();

        backfill_legacy_meetings(&dir.0).unwrap();

        let detail = meeting_in_dir(&dir.0, "express-legacy").unwrap();
        assert_eq!(detail.meeting.title, "legacy");
        assert_eq!(detail.transcript, "Legacy transcript");
    }

    #[test]
    fn recovers_a_claimed_request_after_restart() {
        let dir = TestDir::new();
        initialize_dir(&dir.0).unwrap();
        let media_path = dir.0.join("meeting.m4a");
        fs::write(&media_path, b"audio").unwrap();
        let request = request(&media_path, "express-restart");
        write_request(&processing_path(&dir.0, &request.id), &request);

        let recovered = claim_next_in_dir(&dir.0).unwrap().unwrap();

        assert_eq!(recovered.id, request.id);
    }

    #[test]
    fn ignores_stale_processing_files_with_a_receipt() {
        let dir = TestDir::new();
        initialize_dir(&dir.0).unwrap();
        let media_path = dir.0.join("meeting.m4a");
        fs::write(&media_path, b"audio").unwrap();
        let request = request(&media_path, "express-completed");
        let processing_path = processing_path(&dir.0, &request.id);
        write_request(&processing_path, &request);
        fs::write(
            dir.0.join("completed").join("express-completed.json"),
            b"{}",
        )
        .unwrap();

        assert!(claim_next_in_dir(&dir.0).unwrap().is_none());
        assert!(!processing_path.exists());
    }

    #[test]
    fn completes_a_claimed_request_atomically() {
        let dir = TestDir::new();
        initialize_dir(&dir.0).unwrap();
        let media_path = dir.0.join("meeting.m4a");
        fs::write(&media_path, b"audio").unwrap();
        let request = request(&media_path, "express-complete");
        let processing_path = processing_path(&dir.0, &request.id);
        write_request(&processing_path, &request);
        let result = TranscriptionResult {
            job_id: request.id.clone(),
            text: "Тест".to_string(),
            segments: vec![TranscriptSegment {
                index: 0,
                start_ms: 0,
                end_ms: 1_000,
                text: "Тест".to_string(),
                speaker: None,
            }],
            benchmark: None,
        };

        let completion = complete_in_dir(&dir.0, &request.id, &result).unwrap();

        assert!(!processing_path.exists());
        assert!(dir.0.join("completed/express-complete.json").is_file());
        assert!(dir.0.join("meetings/express-complete.json").is_file());
        assert!(Path::new(&completion.transcript_path).is_file());
        assert!(fs::read_to_string(completion.transcript_path)
            .unwrap()
            .contains("Тест"));

        let meetings = list_meetings_in_dir(&dir.0).unwrap();
        assert_eq!(meetings.len(), 1);
        assert_eq!(meetings[0].title, "Product sync");
        let detail = meeting_in_dir(&dir.0, "express-complete").unwrap();
        assert!(detail.transcript.contains("Тест"));

        let detail = save_summary_in_dir(
            &dir.0,
            "express-complete",
            MeetingSummary {
                text: "Короткое резюме".to_string(),
                model: "gpt-oss-120b".to_string(),
                generated_at: "2026-08-28T12:00:00Z".to_string(),
            },
        )
        .unwrap();
        assert_eq!(detail.meeting.summary.unwrap().model, "gpt-oss-120b");
    }

    #[test]
    fn refuses_a_result_from_another_job() {
        let dir = TestDir::new();
        initialize_dir(&dir.0).unwrap();
        let media_path = dir.0.join("meeting.m4a");
        fs::write(&media_path, b"audio").unwrap();
        let request = request(&media_path, "express-expected");
        let processing_path = processing_path(&dir.0, &request.id);
        write_request(&processing_path, &request);
        let result = TranscriptionResult {
            job_id: "express-other".to_string(),
            text: "Wrong result".to_string(),
            segments: Vec::new(),
            benchmark: None,
        };

        assert!(complete_in_dir(&dir.0, &request.id, &result).is_err());
        assert!(processing_path.is_file());
        assert!(!dir.0.join("express-expected.localscribe.md").exists());
    }
    #[test]
    fn distinct_jobs_keep_their_own_transcripts() {
        let dir = TestDir::new();
        initialize_dir(&dir.0).unwrap();
        for (id, extension, text) in [
            ("first", "mp4", "First recording"),
            ("second", "m4a", "Second recording"),
            ("third", "mp4", "Third recording"),
        ] {
            let media = dir.0.join(format!("meeting.{extension}"));
            fs::write(&media, b"audio").unwrap();
            let request = request(&media, id);
            write_request(&processing_path(&dir.0, id), &request);
            let result = TranscriptionResult {
                job_id: id.to_string(),
                text: text.to_string(),
                segments: Vec::new(),
                benchmark: None,
            };
            complete_in_dir(&dir.0, id, &result).unwrap();
        }
        for (id, text) in [
            ("first", "First recording"),
            ("second", "Second recording"),
            ("third", "Third recording"),
        ] {
            let detail = meeting_in_dir(&dir.0, id).unwrap();
            assert!(
                detail.transcript.contains(text),
                "{id} lost its original transcript"
            );
        }
    }
}
