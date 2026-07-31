use crate::{
    errors::AppResult,
    types::{ExportFormat, ExportRequest, TranscriptSegment},
};
use std::{fs, path::PathBuf};

pub fn export_transcript(request: ExportRequest) -> AppResult<String> {
    let output_path = PathBuf::from(&request.output_path);
    let content = format_transcript(request.format, &request.text, &request.segments);
    fs::write(&output_path, content)?;
    Ok(output_path.to_string_lossy().to_string())
}

pub fn format_transcript(
    format: ExportFormat,
    text: &str,
    segments: &[TranscriptSegment],
) -> String {
    match format {
        ExportFormat::Txt => format_txt(text, segments),
        ExportFormat::Md => format_md(text, segments),
    }
}

fn format_txt(text: &str, segments: &[TranscriptSegment]) -> String {
    if !text.trim().is_empty() {
        return ensure_trailing_newline(text.trim());
    }

    ensure_trailing_newline(
        &segments
            .iter()
            .map(segment_display_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn format_md(text: &str, segments: &[TranscriptSegment]) -> String {
    if segments.is_empty() {
        return format!("# Transcript\n\n{}\n", text.trim());
    }

    let mut output = String::from("# Transcript\n\n");

    for segment in segments {
        if segment.text.trim().is_empty() {
            continue;
        }

        let timestamp = format_timestamp_md(segment.start_ms);
        let text = segment_display_text(segment);

        output.push_str(&format!("## {}\n\n{}\n\n", timestamp, text));
    }

    output
}

fn format_timestamp_md(ms: u64) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1_000;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

fn ensure_trailing_newline(text: &str) -> String {
    if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
}

fn segment_display_text(segment: &TranscriptSegment) -> String {
    let text = segment.text.trim();
    if text.is_empty() {
        return String::new();
    }

    match segment
        .speaker
        .as_deref()
        .map(str::trim)
        .filter(|speaker| !speaker.is_empty())
    {
        Some(speaker) => format!("**{}**: {}", speaker, text),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_md_timestamp() {
        assert_eq!(format_timestamp_md(3_723_045), "01:02:03");
        assert_eq!(format_timestamp_md(120_000), "02:00");
        assert_eq!(format_timestamp_md(5_000), "00:05");
    }

    #[test]
    fn formats_md_segments() {
        let segments = vec![TranscriptSegment {
            index: 0,
            start_ms: 120,
            end_ms: 2_340,
            text: "Hello world".to_string(),
            speaker: None,
        }];
        let result = format_transcript(ExportFormat::Md, "", &segments);
        assert!(result.contains("# Transcript"));
        assert!(result.contains("## 00:00"));
        assert!(result.contains("Hello world"));
    }

    #[test]
    fn formats_md_with_speakers() {
        let segments = vec![TranscriptSegment {
            index: 0,
            start_ms: 120,
            end_ms: 2_340,
            text: "Hello".to_string(),
            speaker: Some("Speaker 1".to_string()),
        }];
        let result = format_transcript(ExportFormat::Md, "", &segments);
        assert!(result.contains("**Speaker 1**: Hello"));
    }
}
