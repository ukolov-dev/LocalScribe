//! Neural speaker segmentation, embeddings and automatic clustering run in an isolated worker.
use crate::{
    errors::{AppError, AppResult},
    types::{TranscriptSegment, TranscriptionResult},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerTurn {
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker: usize,
}

fn failure(message: impl Into<String>) -> AppError {
    AppError::Diarization(message.into())
}

pub fn timed_tokens(
    segment: &whisper_rs::WhisperSegment<'_>,
    end_token: i32,
    offset: u64,
) -> AppResult<Vec<TranscriptSegment>> {
    let mut tokens = Vec::new();
    for i in 0..segment.n_tokens() {
        let Some(token) = segment.get_token(i) else {
            continue;
        };
        if token.token_id() >= end_token {
            continue;
        }
        let data = token.token_data();
        if data.t0 < 0 || data.t1 < data.t0 {
            return Ok(Vec::new());
        }
        tokens.push((
            token.to_bytes()?.to_vec(),
            offset + data.t0 as u64 * 10,
            offset + data.t1 as u64 * 10,
        ));
    }
    Ok(group_tokens(tokens, segment.to_str_lossy()?.trim()))
}

fn group_tokens(tokens: Vec<(Vec<u8>, u64, u64)>, original: &str) -> Vec<TranscriptSegment> {
    let mut pieces: Vec<TranscriptSegment> = Vec::new();
    let mut buffer = Vec::new();
    let (mut start, mut end) = (0, 0);
    for (bytes, t0, t1) in tokens {
        let cjk = std::str::from_utf8(&bytes).ok().and_then(|s| s.chars().next()).is_some_and(|c| matches!(c, '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{9fff}' | '\u{ac00}'..='\u{d7af}'));
        let boundary = bytes.first().is_some_and(u8::is_ascii_whitespace) || cjk;
        if boundary && !buffer.is_empty() && std::str::from_utf8(&buffer).is_ok() {
            pieces.push(TranscriptSegment {
                index: pieces.len() as u32,
                start_ms: start,
                end_ms: end,
                text: String::from_utf8(std::mem::take(&mut buffer)).unwrap(),
                speaker: None,
            });
        }
        if buffer.is_empty() {
            start = t0;
        }
        end = t1;
        buffer.extend(bytes);
    }
    if !buffer.is_empty() {
        let Ok(text) = String::from_utf8(buffer) else {
            return Vec::new();
        };
        pieces.push(TranscriptSegment {
            index: pieces.len() as u32,
            start_ms: start,
            end_ms: end,
            text,
            speaker: None,
        });
    }
    if pieces
        .iter()
        .map(|s| s.text.as_str())
        .collect::<String>()
        .trim()
        != original
    {
        return Vec::new();
    }
    if let Some(first) = pieces.first_mut() {
        if !first.text.starts_with(char::is_whitespace) {
            first.text.insert(0, ' ');
        }
    }
    pieces
}

/// This entry point runs before AppKit/Tauri initialization; native crashes cannot take down the UI.
pub fn run_cli(args: &[String]) -> AppResult<()> {
    #[cfg(unix)]
    {
        unsafe extern "C" {
            fn getppid() -> i32;
        }
        let parent = unsafe { getppid() };
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(500));
            if unsafe { getppid() } != parent {
                std::process::exit(1);
            }
        });
    }
    use sherpa_onnx::{OfflineSpeakerDiarization, OfflineSpeakerDiarizationConfig, Wave};
    if args.len() != 3 {
        return Err(failure(
            "Expected segmentation model, embedding model and WAV.",
        ));
    }
    let mut config = OfflineSpeakerDiarizationConfig::default();
    config.segmentation.pyannote.model = Some(args[0].clone());
    config.segmentation.num_threads = 2;
    config.embedding.model = Some(args[1].clone());
    config.embedding.num_threads = 2;
    // Auto clustering: -1 deliberately leaves the total number of speakers unconstrained.
    config.clustering.num_clusters = -1;
    config.clustering.threshold = 0.9;
    let engine = OfflineSpeakerDiarization::create(&config)
        .ok_or_else(|| failure("Cannot load speaker models."))?;
    let wav = Wave::read(&args[2]).ok_or_else(|| failure("Cannot read diarization WAV."))?;
    if wav.sample_rate() != engine.sample_rate() || wav.samples().len() > i32::MAX as usize {
        return Err(failure(
            "Invalid diarization sample rate or recording too long.",
        ));
    }
    let result = engine
        .process(wav.samples())
        .ok_or_else(|| failure("Speaker inference failed."))?;
    let mut names = HashMap::new();
    let mut turns = Vec::new();
    for s in result.sort_by_start_time() {
        if !s.start.is_finite()
            || !s.end.is_finite()
            || s.start < 0.0
            || s.end <= s.start
            || s.speaker < 0
        {
            continue;
        }
        let next = names.len() + 1;
        let speaker = *names.entry(s.speaker).or_insert(next);
        turns.push(SpeakerTurn {
            start_ms: (s.start as f64 * 1000.0).round() as u64,
            end_ms: (s.end as f64 * 1000.0).round() as u64,
            speaker,
        });
    }
    println!("{}", serde_json::to_string(&turns)?);
    Ok(())
}

struct Worker {
    child: Child,
    output: std::path::PathBuf,
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.output);
    }
}

pub fn detect_speakers(
    wav: &Path,
    model_dir: &Path,
    cancelled: &AtomicBool,
) -> AppResult<Vec<SpeakerTurn>> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(AppError::Cancelled);
    }
    let output = wav.with_extension("speakers.json");
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(&output)?;
    let child = Command::new(std::env::current_exe()?)
        .arg("--diarize")
        .arg(model_dir.join("segmentation.onnx"))
        .arg(model_dir.join("embedding.onnx"))
        .arg(wav)
        .stdin(Stdio::null())
        .stdout(Stdio::from(file))
        .stderr(Stdio::null())
        .spawn();
    let child = match child {
        Ok(child) => child,
        Err(error) => {
            let _ = fs::remove_file(output);
            return Err(error.into());
        }
    };
    let mut worker = Worker { child, output };
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::Cancelled);
        }
        if let Some(status) = worker.child.try_wait()? {
            if !status.success() {
                return Err(failure(
                    "Движок спикеров завершился с ошибкой. Проверьте модели и повторите попытку.",
                ));
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let turns: Vec<SpeakerTurn> = serde_json::from_slice(&fs::read(&worker.output)?)?;
    if turns
        .iter()
        .any(|s| s.end_ms <= s.start_ms || s.speaker == 0)
    {
        return Err(failure("Invalid speaker timeline."));
    }
    Ok(turns)
}

pub fn assign_speakers(result: &mut TranscriptionResult, turns: &[SpeakerTurn]) {
    let mut merged: Vec<TranscriptSegment> = Vec::new();
    for mut segment in std::mem::take(&mut result.segments) {
        if segment.text.trim().is_empty() {
            continue;
        }
        segment.speaker =
            speaker_at(segment.start_ms, segment.end_ms, turns).map(|id| format!("Speaker {id}"));
        if segment.speaker.is_none() {
            segment.speaker = Some("Неизвестный спикер".into());
        }
        if let Some(last) = merged.last_mut() {
            if last.speaker == segment.speaker
                && segment.start_ms <= last.end_ms.saturating_add(1500)
                && segment.end_ms.saturating_sub(last.start_ms) <= 15_000
            {
                // Token pieces retain their original whitespace, including punctuation and CJK.
                last.text.push_str(&segment.text);
                last.end_ms = last.end_ms.max(segment.end_ms);
                continue;
            }
        }
        segment.index = merged.len() as u32;
        merged.push(segment);
    }
    for s in &mut merged {
        s.text = s.text.trim().to_string();
    }
    result.segments = merged;
}

fn speaker_at(start: u64, end: u64, turns: &[SpeakerTurn]) -> Option<usize> {
    let end = end.max(start.saturating_add(1));
    let mut durations: HashMap<usize, u64> = HashMap::new();
    for turn in turns {
        let overlap = end
            .min(turn.end_ms)
            .saturating_sub(start.max(turn.start_ms));
        *durations.entry(turn.speaker).or_default() += overlap;
    }
    let best = durations
        .into_iter()
        .filter(|(_, duration)| *duration > 0)
        .max_by_key(|(id, duration)| (*duration, std::cmp::Reverse(*id)));
    if let Some((id, _)) = best {
        return Some(id);
    }
    // Timestamp estimates can land in a short VAD gap; never invent a voice for distant speech.
    turns
        .iter()
        .map(|s| {
            (
                s.start_ms
                    .saturating_sub(end)
                    .max(start.saturating_sub(s.end_ms)),
                s.speaker,
            )
        })
        .min_by_key(|&(gap, id)| (gap, id))
        .filter(|(gap, _)| *gap <= 500)
        .map(|(_, id)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn token_alignment_preserves_split_utf8_words_punctuation_and_speaker_changes() {
        let tokens = vec![
            (vec![32, 0xd0], 0, 50),
            (vec![0x94, 0xd0, 0xb0], 50, 200),
            (b",".to_vec(), 200, 220),
            (" нет".as_bytes().to_vec(), 1000, 1400),
            (b"!".to_vec(), 1400, 1500),
        ];
        let pieces = group_tokens(tokens, "Да, нет!");
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0].text, " Да,");
        assert_eq!(pieces[1].start_ms, 1000);
        let mut result = TranscriptionResult {
            job_id: "test".into(),
            text: String::new(),
            segments: pieces,
            benchmark: None,
        };
        assign_speakers(
            &mut result,
            &[
                SpeakerTurn {
                    start_ms: 0,
                    end_ms: 500,
                    speaker: 1,
                },
                SpeakerTurn {
                    start_ms: 1000,
                    end_ms: 1600,
                    speaker: 3,
                },
            ],
        );
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[1].speaker.as_deref(), Some("Speaker 3"));
        assert_eq!(result.segments[1].text, "нет!");
        assert!(group_tokens(vec![(b"wrong".to_vec(), 0, 100)], "original").is_empty());
    }
    #[test]
    fn cancellation_does_not_launch_a_worker() {
        assert!(matches!(
            detect_speakers(
                Path::new("/missing.wav"),
                Path::new("/models"),
                &AtomicBool::new(true)
            ),
            Err(AppError::Cancelled)
        ));
    }
    #[test]
    fn keeps_four_voices_and_returning_speaker_and_exact_text() {
        let turns: Vec<_> = [1, 2, 3, 4, 1]
            .iter()
            .enumerate()
            .map(|(i, &speaker)| SpeakerTurn {
                start_ms: i as u64 * 1000,
                end_ms: (i + 1) as u64 * 1000,
                speaker,
            })
            .collect();
        let mut result = TranscriptionResult {
            job_id: "test".into(),
            text: String::new(),
            benchmark: None,
            segments: [" Один", " Два", " 三", " Four", " снова"]
                .iter()
                .enumerate()
                .map(|(i, text)| TranscriptSegment {
                    index: i as u32,
                    start_ms: i as u64 * 1000,
                    end_ms: (i + 1) as u64 * 1000,
                    text: text.to_string(),
                    speaker: None,
                })
                .collect(),
        };
        assign_speakers(&mut result, &turns);
        assert_eq!(
            result
                .segments
                .iter()
                .map(|s| s.speaker.as_deref().unwrap())
                .collect::<Vec<_>>(),
            [
                "Speaker 1",
                "Speaker 2",
                "Speaker 3",
                "Speaker 4",
                "Speaker 1"
            ]
        );
        assert_eq!(result.segments[2].text, "三");
    }
    #[test]
    fn sums_overlap_per_voice_and_does_not_invent_speakers_in_silence() {
        let turns = vec![
            SpeakerTurn {
                start_ms: 0,
                end_ms: 400,
                speaker: 1,
            },
            SpeakerTurn {
                start_ms: 400,
                end_ms: 1000,
                speaker: 2,
            },
            SpeakerTurn {
                start_ms: 1000,
                end_ms: 1400,
                speaker: 1,
            },
        ];
        assert_eq!(speaker_at(0, 1400, &turns), Some(1));
        assert_eq!(speaker_at(10_000, 11_000, &turns), None);
        assert_eq!(speaker_at(0, 100, &[]), None);
    }
}
