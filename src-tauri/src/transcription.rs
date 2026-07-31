use crate::{
    audio,
    errors::{display_path, AppError, AppResult},
    models,
    types::{
        BenchmarkPhase, TranscribeOptions, TranscriptSegment, TranscriptionBenchmark,
        TranscriptionProgress, TranscriptionResult, TranscriptionSegmentEvent,
    },
};
use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use whisper_rs::{
    FullParams, SamplingStrategy, SegmentCallbackData, WhisperContext, WhisperContextParameters,
};

const MULTILINGUAL_PROMPT: &str =
    "Русская речь -> русский текст. English speech -> English text. Не переводить. Do not translate.";
const SAMPLE_RATE_HZ: usize = 16_000;
const MULTILINGUAL_QUALITY_CHUNK_SECONDS: usize = 10;
const MULTILINGUAL_QUALITY_OVERLAP_SECONDS: usize = 1;
const MAX_CONSECUTIVE_REPEATED_SEGMENTS: usize = 2;
const MAX_REPEATED_LONG_SEGMENTS: usize = 2;
const MAX_REPEATED_SHORT_SEGMENTS: usize = 8;
const MAX_REPEATED_ACKNOWLEDGEMENTS: usize = 4;

#[derive(Clone, Copy)]
struct RunConfig {
    timestamp_offset_ms: u64,
    segment_index_offset: u32,
    progress_start: f32,
    progress_span: f32,
    emit_start_ms: Option<u64>,
    emit_end_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MultilingualChunk {
    decode_start: usize,
    decode_end: usize,
    output_start: usize,
    output_end: usize,
}

struct ChunkTranscriptionParams<'a> {
    language: String,
    decoding_preset: Option<&'a str>,
    timestamps: bool,
    threads: Option<u32>,
    initial_prompt: Option<&'a str>,
}

struct TranscriptionCallbacks {
    progress_app: AppHandle,
    progress_job_id: String,
    segment_app: AppHandle,
    segment_job_id: String,
    abort_flag: Arc<AtomicBool>,
}

impl RunConfig {
    fn single_pass() -> Self {
        Self {
            timestamp_offset_ms: 0,
            segment_index_offset: 0,
            progress_start: 20.0,
            progress_span: 78.0,
            emit_start_ms: None,
            emit_end_ms: None,
        }
    }
}

impl TranscriptionCallbacks {
    fn new(app: AppHandle, job_id: String, abort_flag: Arc<AtomicBool>) -> Self {
        Self {
            progress_app: app.clone(),
            progress_job_id: job_id.clone(),
            segment_app: app,
            segment_job_id: job_id,
            abort_flag,
        }
    }
}

fn calculate_adaptive_chunk_size(language: &str, _total_samples: usize) -> usize {
    match language.trim().to_ascii_lowercase().as_str() {
        "multi" | "multilingual" | "multi-quality" | "multilingual-quality" => {
            MULTILINGUAL_QUALITY_CHUNK_SECONDS
        }
        "multi-fast" | "multilingual-fast" => 8,
        _ => 30,
    }
}

fn multilingual_overlap_seconds(language: &str) -> usize {
    match language.trim().to_ascii_lowercase().as_str() {
        "multi" | "multilingual" | "multi-quality" | "multilingual-quality" => {
            MULTILINGUAL_QUALITY_OVERLAP_SECONDS
        }
        _ => 0,
    }
}

fn build_multilingual_chunk_plan(
    total_samples: usize,
    chunk_seconds: usize,
    overlap_seconds: usize,
) -> Vec<MultilingualChunk> {
    if total_samples == 0 {
        return Vec::new();
    }

    let chunk_size = SAMPLE_RATE_HZ * chunk_seconds.max(1);
    let overlap_size = (SAMPLE_RATE_HZ * overlap_seconds).min(chunk_size / 2);
    let chunk_count = ((total_samples + chunk_size - 1) / chunk_size).max(1);
    let mut chunks = Vec::with_capacity(chunk_count);

    for chunk_index in 0..chunk_count {
        let output_start = chunk_index * chunk_size;
        let output_end = ((chunk_index + 1) * chunk_size).min(total_samples);
        if output_start >= output_end {
            continue;
        }

        chunks.push(MultilingualChunk {
            decode_start: output_start.saturating_sub(overlap_size),
            decode_end: output_end.saturating_add(overlap_size).min(total_samples),
            output_start,
            output_end,
        });
    }

    chunks
}

fn crop_segments_to_chunk_output(
    segments: Vec<TranscriptSegment>,
    chunk: &MultilingualChunk,
    first_index: u32,
) -> Vec<TranscriptSegment> {
    let output_start_ms = samples_duration_ms(chunk.output_start);
    let output_end_ms = samples_duration_ms(chunk.output_end);
    let mut next_index = first_index;
    let mut cropped = Vec::with_capacity(segments.len());

    for mut segment in segments {
        let midpoint_ms = segment.start_ms + segment.end_ms.saturating_sub(segment.start_ms) / 2;
        if midpoint_ms < output_start_ms || midpoint_ms >= output_end_ms {
            continue;
        }

        segment.index = next_index;
        next_index += 1;
        cropped.push(segment);
    }

    cropped
}

fn build_initial_prompt_opt(
    prompt: Option<&str>,
    preserve_multilingual: bool,
) -> AppResult<Option<String>> {
    let user_prompt = prompt.map(str::trim).filter(|p| !p.is_empty());

    let result = match (preserve_multilingual, user_prompt) {
        (true, Some(user_prompt)) => format!("{MULTILINGUAL_PROMPT}\n{user_prompt}"),
        (true, None) => MULTILINGUAL_PROMPT.to_string(),
        (false, Some(user_prompt)) => user_prompt.to_string(),
        (false, None) => return Ok(None),
    };

    if result.contains('\0') {
        return Err(AppError::InvalidInput(
            "Initial prompt cannot contain null bytes.".to_string(),
        ));
    }

    Ok(Some(result))
}

pub fn transcribe(
    app: AppHandle,
    job_id: String,
    options: TranscribeOptions,
    cancel_flag: Arc<AtomicBool>,
) -> AppResult<TranscriptionResult> {
    let total_start = Instant::now();
    let mut benchmark_phases = Vec::new();
    let input_path = PathBuf::from(&options.file_path);
    let model_path = models::resolve_model_path(&app, &options)?;

    emit_progress(&app, &job_id, "preparing-audio", 4.0)?;
    check_cancelled(&cancel_flag)?;

    let audio_start = Instant::now();
    let prepared_audio = audio::prepare_audio(&app, &job_id, &input_path)?;
    push_benchmark_phase(
        &mut benchmark_phases,
        "audio-preparation",
        audio_start.elapsed(),
    );
    emit_progress(&app, &job_id, "loading-model", 15.0)?;

    let prefer_gpu = should_use_gpu(&model_path, &options);
    let result = if is_chunked_multilingual_language(&options.language) {
        match transcribe_multilingual_samples(
            &app,
            &job_id,
            &model_path,
            &prepared_audio.samples,
            &options,
            Arc::clone(&cancel_flag),
            prefer_gpu,
            &mut benchmark_phases,
        ) {
            Ok(result) => Ok(result),
            Err(error) if prefer_gpu && should_retry_gpu_on_cpu(&error, &options) => {
                emit_progress(&app, &job_id, "retrying-cpu", 18.0)?;
                transcribe_multilingual_samples(
                    &app,
                    &job_id,
                    &model_path,
                    &prepared_audio.samples,
                    &options,
                    Arc::clone(&cancel_flag),
                    false,
                    &mut benchmark_phases,
                )
            }
            Err(error) => Err(error),
        }
    } else if prefer_gpu {
        match transcribe_samples(
            &app,
            &job_id,
            &model_path,
            &prepared_audio.samples,
            &options,
            Arc::clone(&cancel_flag),
            true,
            &mut benchmark_phases,
        ) {
            Ok(result) => Ok(result),
            Err(error) if should_retry_gpu_on_cpu(&error, &options) => {
                emit_progress(&app, &job_id, "retrying-cpu", 18.0)?;
                transcribe_samples(
                    &app,
                    &job_id,
                    &model_path,
                    &prepared_audio.samples,
                    &options,
                    Arc::clone(&cancel_flag),
                    false,
                    &mut benchmark_phases,
                )
            }
            Err(error) => Err(error),
        }
    } else {
        transcribe_samples(
            &app,
            &job_id,
            &model_path,
            &prepared_audio.samples,
            &options,
            Arc::clone(&cancel_flag),
            false,
            &mut benchmark_phases,
        )
    };
    let result = retry_auto_language_if_empty(
        &app,
        &job_id,
        &model_path,
        &prepared_audio.samples,
        &options,
        Arc::clone(&cancel_flag),
        result,
        &mut benchmark_phases,
    );
    audio::cleanup_temp_file(&prepared_audio.wav_path);

    let mut result = result?;
    if should_assign_speakers(&options) {
        emit_progress(&app, &job_id, "assigning-speakers", 99.0)?;
        let speakers_start = Instant::now();
        crate::diarization::assign_speakers(&mut result, &prepared_audio.samples);
        push_benchmark_phase(
            &mut benchmark_phases,
            "speaker-assignment",
            speakers_start.elapsed(),
        );
        result.text = transcript_text(&result.segments);
    }

    result.benchmark = Some(TranscriptionBenchmark {
        total_ms: duration_ms(total_start.elapsed()),
        audio_duration_ms: samples_duration_ms(prepared_audio.samples.len()),
        phases: benchmark_phases,
    });

    app.emit("transcription-completed", result.clone())?;
    Ok(result)
}

fn transcribe_samples(
    app: &AppHandle,
    job_id: &str,
    model_path: &PathBuf,
    samples: &[f32],
    options: &TranscribeOptions,
    cancel_flag: Arc<AtomicBool>,
    use_gpu: bool,
    benchmark_phases: &mut Vec<BenchmarkPhase>,
) -> AppResult<TranscriptionResult> {
    let load_start = Instant::now();
    let context = load_context(model_path, use_gpu)?;
    push_benchmark_phase(
        benchmark_phases,
        if use_gpu {
            "model-load-metal"
        } else {
            "model-load-cpu"
        },
        load_start.elapsed(),
    );

    let _estimated_segments = (samples.len() / (SAMPLE_RATE_HZ * 10)) * 3;
    let callbacks =
        TranscriptionCallbacks::new(app.clone(), job_id.to_string(), cancel_flag.clone());

    let chunk_params = ChunkTranscriptionParams {
        language: options.language.clone(),
        decoding_preset: options.decoding_preset.as_deref(),
        timestamps: options.timestamps,
        threads: options.threads,
        initial_prompt: options.initial_prompt.as_deref(),
    };

    let whisper_start = Instant::now();
    let result = transcribe_with_context_optimized(
        &context,
        samples,
        &chunk_params,
        &callbacks,
        RunConfig::single_pass(),
    );
    push_benchmark_phase(
        benchmark_phases,
        if use_gpu {
            "whisper-metal"
        } else {
            "whisper-cpu"
        },
        whisper_start.elapsed(),
    );
    result
}

fn transcribe_multilingual_samples(
    app: &AppHandle,
    job_id: &str,
    model_path: &PathBuf,
    samples: &[f32],
    options: &TranscribeOptions,
    cancel_flag: Arc<AtomicBool>,
    use_gpu: bool,
    benchmark_phases: &mut Vec<BenchmarkPhase>,
) -> AppResult<TranscriptionResult> {
    check_cancelled(&cancel_flag)?;
    let load_start = Instant::now();
    let context = load_context(model_path, use_gpu)?;
    push_benchmark_phase(
        benchmark_phases,
        if use_gpu {
            "model-load-metal"
        } else {
            "model-load-cpu"
        },
        load_start.elapsed(),
    );

    let chunk_size_seconds = calculate_adaptive_chunk_size(&options.language, samples.len());
    let overlap_seconds = multilingual_overlap_seconds(&options.language);
    let chunks = build_multilingual_chunk_plan(samples.len(), chunk_size_seconds, overlap_seconds);
    let chunk_count = chunks.len().max(1);

    let estimated_segments = (samples.len() / (SAMPLE_RATE_HZ * 10)) * 3;
    let mut segments = Vec::with_capacity(estimated_segments);
    let mut next_segment_index = 0;

    let callbacks =
        TranscriptionCallbacks::new(app.clone(), job_id.to_string(), cancel_flag.clone());
    let initial_prompt_ref = options.initial_prompt.as_deref();

    let chunk_params = ChunkTranscriptionParams {
        language: "auto".to_string(),
        decoding_preset: options.decoding_preset.as_deref(),
        timestamps: options.timestamps,
        threads: options.threads,
        initial_prompt: initial_prompt_ref,
    };

    let whisper_start = Instant::now();
    for (chunk_index, chunk) in chunks.iter().enumerate() {
        check_cancelled(&cancel_flag)?;

        if chunk.decode_start >= chunk.decode_end {
            continue;
        }

        let progress_start = 20.0 + (chunk_index as f32 / chunk_count as f32) * 78.0;
        let progress_span = 78.0 / chunk_count as f32;
        let timestamp_offset_ms = samples_duration_ms(chunk.decode_start);

        let run = RunConfig {
            timestamp_offset_ms,
            segment_index_offset: next_segment_index,
            progress_start,
            progress_span,
            emit_start_ms: Some(samples_duration_ms(chunk.output_start)),
            emit_end_ms: Some(samples_duration_ms(chunk.output_end)),
        };

        match transcribe_with_context_optimized(
            &context,
            &samples[chunk.decode_start..chunk.decode_end],
            &chunk_params,
            &callbacks,
            run,
        ) {
            Ok(result) => {
                let cropped =
                    crop_segments_to_chunk_output(result.segments, chunk, next_segment_index);
                next_segment_index += cropped.len() as u32;
                segments.extend(cropped);
            }
            Err(AppError::EmptyTranscript) => continue,
            Err(error) => return Err(error),
        }
    }
    push_benchmark_phase(
        benchmark_phases,
        if use_gpu {
            "whisper-chunked-metal"
        } else {
            "whisper-chunked-cpu"
        },
        whisper_start.elapsed(),
    );

    let segments = filter_hallucinated_segments(segments);
    let text = transcript_text(&segments);
    if text.trim().is_empty() {
        return Err(AppError::EmptyTranscript);
    }

    emit_progress(app, job_id, "completed", 100.0)?;
    Ok(TranscriptionResult {
        job_id: job_id.to_string(),
        text,
        segments,
        benchmark: None,
    })
}

fn load_context(model_path: &PathBuf, use_gpu: bool) -> AppResult<WhisperContext> {
    let mut context_params = WhisperContextParameters::default();
    context_params.use_gpu = use_gpu;
    context_params.gpu_device = 0;

    WhisperContext::new_with_params(model_path, context_params).map_err(|error| {
        AppError::Whisper(format!(
            "Failed to load model {}: {error}",
            display_path(model_path)
        ))
    })
}

fn transcribe_with_context_optimized(
    context: &WhisperContext,
    samples: &[f32],
    params: &ChunkTranscriptionParams,
    callbacks: &TranscriptionCallbacks,
    run: RunConfig,
) -> AppResult<TranscriptionResult> {
    check_cancelled(&callbacks.abort_flag)?;
    let mut state = context.create_state()?;

    let mut full_params = FullParams::new(sampling_strategy_for_decoding(params.decoding_preset));
    full_params.set_n_threads(resolve_thread_count(params.threads));
    full_params.set_print_progress(false);
    full_params.set_print_special(false);
    full_params.set_print_realtime(false);
    full_params.set_print_timestamps(false);
    full_params.set_debug_mode(false);
    full_params.set_translate(false);
    full_params.set_no_timestamps(!params.timestamps);
    full_params.set_suppress_blank(true);
    full_params.set_suppress_nst(true);
    full_params.set_temperature(0.0);
    full_params.set_temperature_inc(0.0);
    full_params.set_entropy_thold(2.2);
    full_params.set_logprob_thold(-1.0);
    full_params.set_no_speech_thold(0.6);

    let normalized_language = params.language.trim().to_ascii_lowercase();
    let auto_or_multilingual =
        is_auto_language(&normalized_language) || is_multilingual_language(&normalized_language);
    full_params.set_no_context(auto_or_multilingual);

    if auto_or_multilingual {
        full_params.set_language(Some("auto"));
        full_params.set_detect_language(false);
    } else {
        full_params.set_language(Some(&normalized_language));
        full_params.set_detect_language(false);
    }

    let prompt = build_initial_prompt_opt(params.initial_prompt, auto_or_multilingual)?;
    if let Some(prompt) = prompt.as_deref() {
        full_params.set_initial_prompt(prompt);
    }

    let progress_start = run.progress_start;
    let progress_span = run.progress_span;
    let progress_app = callbacks.progress_app.clone();
    let progress_job_id = callbacks.progress_job_id.clone();

    full_params.set_progress_callback_safe::<_, Box<dyn FnMut(i32)>>(Some(Box::new(
        move |progress: i32| {
            let mapped = progress_start + (progress as f32 * progress_span / 100.0);
            let _ = emit_progress(&progress_app, &progress_job_id, "transcribing", mapped);
        },
    )
        as Box<dyn FnMut(i32)>));

    let segment_app = callbacks.segment_app.clone();
    let segment_job_id = callbacks.segment_job_id.clone();
    let timestamp_offset_ms = run.timestamp_offset_ms;
    let segment_index_offset = run.segment_index_offset;
    let emit_start_ms = run.emit_start_ms;
    let emit_end_ms = run.emit_end_ms;

    full_params.set_segment_callback_safe_lossy::<_, Box<dyn FnMut(SegmentCallbackData)>>(Some(
        Box::new(move |segment: SegmentCallbackData| {
            let start_ms = timestamp_offset_ms + centiseconds_to_ms(segment.start_timestamp);
            let end_ms = timestamp_offset_ms + centiseconds_to_ms(segment.end_timestamp);
            if let (Some(emit_start_ms), Some(emit_end_ms)) = (emit_start_ms, emit_end_ms) {
                let midpoint_ms = start_ms + end_ms.saturating_sub(start_ms) / 2;
                if midpoint_ms < emit_start_ms || midpoint_ms >= emit_end_ms {
                    return;
                }
            }

            let _ = segment_app.emit(
                "transcription-segment",
                TranscriptionSegmentEvent {
                    job_id: segment_job_id.clone(),
                    segment: TranscriptSegment {
                        index: segment_index_offset + segment.segment.max(0) as u32,
                        start_ms,
                        end_ms,
                        text: segment.text.trim().to_string(),
                        speaker: None,
                    },
                },
            );
        }) as Box<dyn FnMut(SegmentCallbackData)>,
    ));

    let abort_flag = Arc::clone(&callbacks.abort_flag);
    full_params.set_abort_callback_safe::<_, Box<dyn FnMut() -> bool>>(Some(Box::new(move || {
        abort_flag.load(Ordering::Relaxed)
    })
        as Box<dyn FnMut() -> bool>));

    state.full(full_params, samples)?;
    if callbacks.abort_flag.load(Ordering::Relaxed) {
        return Err(AppError::Cancelled);
    }

    let num_segments = state.full_n_segments();
    let mut segments = Vec::with_capacity(num_segments as usize);

    for index in 0..num_segments {
        if let Some(segment) = state.get_segment(index) {
            let text = segment.to_str_lossy()?.trim().to_string();
            if text.is_empty() {
                continue;
            }

            segments.push(TranscriptSegment {
                index: run.segment_index_offset + index as u32,
                start_ms: run.timestamp_offset_ms + centiseconds_to_ms(segment.start_timestamp()),
                end_ms: run.timestamp_offset_ms + centiseconds_to_ms(segment.end_timestamp()),
                text,
                speaker: None,
            });
        }
    }

    let segments = filter_hallucinated_segments(segments);
    let text = transcript_text(&segments);

    if text.trim().is_empty() {
        return Err(AppError::EmptyTranscript);
    }

    Ok(TranscriptionResult {
        job_id: callbacks.segment_job_id.clone(),
        text,
        segments,
        benchmark: None,
    })
}

fn transcript_text(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .map(segment_display_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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
        Some(speaker) => format!("{speaker}: {text}"),
        None => text.to_string(),
    }
}

fn filter_hallucinated_segments(segments: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    let mut filtered = Vec::with_capacity(segments.len());
    let mut seen_counts: HashMap<String, usize> = HashMap::new();
    let mut last_key = String::new();
    let mut consecutive_count = 0;

    for mut segment in segments {
        let text = segment.text.trim();
        let key = normalize_repetition_key(text);
        if key.is_empty() {
            continue;
        }

        if is_known_caption_hallucination(&key) {
            continue;
        }

        if key == last_key {
            consecutive_count += 1;
        } else {
            last_key = key.clone();
            consecutive_count = 1;
        }

        let word_count = key.split_whitespace().count();
        let seen_key = repetition_seen_key(&key);
        let seen_count = seen_counts.get(seen_key).copied().unwrap_or(0);
        let is_short = word_count <= 3;
        let max_seen = if is_acknowledgement_key(&key) {
            MAX_REPEATED_ACKNOWLEDGEMENTS
        } else if is_short {
            MAX_REPEATED_SHORT_SEGMENTS
        } else {
            MAX_REPEATED_LONG_SEGMENTS
        };

        if consecutive_count > MAX_CONSECUTIVE_REPEATED_SEGMENTS
            || seen_count >= max_seen
            || has_runaway_repetition(&key)
        {
            continue;
        }

        segment.index = filtered.len() as u32;
        segment.text = text.to_string();
        filtered.push(segment);
        *seen_counts.entry(seen_key.to_string()).or_default() += 1;
    }

    filtered
}

fn is_known_caption_hallucination(key: &str) -> bool {
    matches!(key, "музыка" | "music")
        || key.starts_with("субтитры создавал")
        || key.starts_with("субтитры создал")
        || key.starts_with("субтитры сделал")
        || key.starts_with("субтитры подготовил")
        || key.starts_with("subtitles by")
        || key.starts_with("subtitle by")
        || key.starts_with("subtitles created by")
        || key.starts_with("subtitle created by")
}

fn repetition_seen_key<'a>(key: &'a str) -> &'a str {
    if is_acknowledgement_key(key) {
        "__acknowledgement__"
    } else {
        key
    }
}

fn is_acknowledgement_key(key: &str) -> bool {
    matches!(
        key,
        "угу" | "ага" | "mm hmm" | "mhm" | "mmhmm" | "uh huh" | "uhhuh"
    )
}

fn normalize_repetition_key(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut previous_was_space = true;

    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            normalized.push(character);
            previous_was_space = false;
        } else if !previous_was_space {
            normalized.push(' ');
            previous_was_space = true;
        }
    }

    normalized.trim().to_string()
}

fn has_runaway_repetition(key: &str) -> bool {
    let words = key.split_whitespace().collect::<Vec<_>>();
    if words.len() < 6 {
        return false;
    }

    let max_ngram = 4.min(words.len() / 2);
    for ngram_size in 1..=max_ngram {
        let required_repeats = if ngram_size == 1 { 4 } else { 3 };
        let mut index = 0;

        while index + (ngram_size * required_repeats) <= words.len() {
            let phrase = &words[index..index + ngram_size];
            let mut repeats = 1;

            while index + ((repeats + 1) * ngram_size) <= words.len()
                && words[index + repeats * ngram_size..index + (repeats + 1) * ngram_size]
                    == *phrase
            {
                repeats += 1;
            }

            if repeats >= required_repeats {
                return true;
            }

            index += repeats.max(1) * ngram_size;
        }
    }

    false
}

fn should_assign_speakers(options: &TranscribeOptions) -> bool {
    options
        .speaker_mode
        .as_deref()
        .map(str::trim)
        .is_some_and(|mode| mode.eq_ignore_ascii_case("auto"))
}

fn should_use_gpu(_model_path: &PathBuf, options: &TranscribeOptions) -> bool {
    engine_should_use_gpu(
        options.engine.as_deref(),
        env::var("LOCALSCRIBE_DISABLE_METAL").as_deref() == Ok("1"),
        env::var("LOCALSCRIBE_FORCE_METAL").as_deref() == Ok("1"),
    )
}

fn engine_should_use_gpu(engine: Option<&str>, metal_disabled: bool, metal_forced: bool) -> bool {
    if metal_disabled {
        return false;
    }
    if metal_forced {
        return true;
    }

    !matches!(
        engine.map(|value| value.trim().to_ascii_lowercase()),
        Some(value) if value == "cpu"
    )
}

fn should_retry_on_cpu(error: &AppError) -> bool {
    matches!(error, AppError::Whisper(_) | AppError::EmptyTranscript)
}

fn should_retry_gpu_on_cpu(error: &AppError, options: &TranscribeOptions) -> bool {
    should_retry_on_cpu(error)
        && engine_allows_cpu_fallback(
            options.engine.as_deref(),
            env::var("LOCALSCRIBE_FORCE_METAL").as_deref() == Ok("1"),
        )
}

fn engine_allows_cpu_fallback(engine: Option<&str>, metal_forced: bool) -> bool {
    if metal_forced {
        return false;
    }

    !matches!(
        engine.map(|value| value.trim().to_ascii_lowercase()),
        Some(value) if value == "cpu"
    )
}

fn sampling_strategy_for_decoding(decoding_preset: Option<&str>) -> SamplingStrategy {
    match decoding_preset
        .unwrap_or("balanced")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "accurate" | "beam" | "beam-search" => SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        },
        _ => SamplingStrategy::Greedy { best_of: 1 },
    }
}

fn retry_auto_language_if_empty(
    app: &AppHandle,
    job_id: &str,
    model_path: &PathBuf,
    samples: &[f32],
    options: &TranscribeOptions,
    cancel_flag: Arc<AtomicBool>,
    result: AppResult<TranscriptionResult>,
    benchmark_phases: &mut Vec<BenchmarkPhase>,
) -> AppResult<TranscriptionResult> {
    if !is_auto_language(&options.language) || !matches!(result, Err(AppError::EmptyTranscript)) {
        return result;
    }

    for language in ["ru", "en"] {
        let mut retry_options = options.clone();
        retry_options.language = language.to_string();
        emit_progress(app, job_id, &format!("retrying-{language}"), 18.0)?;
        let retry_use_gpu = should_use_gpu(model_path, &retry_options);

        match transcribe_samples(
            app,
            job_id,
            model_path,
            samples,
            &retry_options,
            Arc::clone(&cancel_flag),
            retry_use_gpu,
            benchmark_phases,
        ) {
            Ok(result) => return Ok(result),
            Err(error) if retry_use_gpu && should_retry_gpu_on_cpu(&error, &retry_options) => {
                emit_progress(app, job_id, "retrying-cpu", 18.0)?;
                match transcribe_samples(
                    app,
                    job_id,
                    model_path,
                    samples,
                    &retry_options,
                    Arc::clone(&cancel_flag),
                    false,
                    benchmark_phases,
                ) {
                    Ok(result) => return Ok(result),
                    Err(AppError::EmptyTranscript) => continue,
                    Err(error) => return Err(error),
                }
            }
            Err(AppError::EmptyTranscript) => continue,
            Err(error) => return Err(error),
        }
    }

    result
}

fn is_auto_language(language: &str) -> bool {
    let normalized = language.trim().to_ascii_lowercase();
    normalized.is_empty() || normalized == "auto"
}

fn is_multilingual_language(language: &str) -> bool {
    matches!(
        language.trim().to_ascii_lowercase().as_str(),
        "multi"
            | "multilingual"
            | "multi-quality"
            | "multilingual-quality"
            | "multi-fast"
            | "multilingual-fast"
    )
}

fn is_chunked_multilingual_language(language: &str) -> bool {
    matches!(
        language.trim().to_ascii_lowercase().as_str(),
        "multi" | "multilingual" | "multi-quality" | "multilingual-quality"
    )
}

fn resolve_thread_count(requested: Option<u32>) -> i32 {
    let available = std::thread::available_parallelism()
        .map(|threads| threads.get() as u32)
        .unwrap_or(4);
    requested.unwrap_or(available).clamp(1, available.max(1)) as i32
}

fn emit_progress(app: &AppHandle, job_id: &str, phase: &str, progress: f32) -> AppResult<()> {
    app.emit(
        "transcription-progress",
        TranscriptionProgress {
            job_id: job_id.to_string(),
            phase: phase.to_string(),
            progress: progress.clamp(0.0, 100.0),
        },
    )?;
    Ok(())
}

fn check_cancelled(cancel_flag: &AtomicBool) -> AppResult<()> {
    if cancel_flag.load(Ordering::Relaxed) {
        Err(AppError::Cancelled)
    } else {
        Ok(())
    }
}

fn centiseconds_to_ms(value: i64) -> u64 {
    value.max(0) as u64 * 10
}

fn push_benchmark_phase(phases: &mut Vec<BenchmarkPhase>, phase: &str, duration: Duration) {
    phases.push(BenchmarkPhase {
        phase: phase.to_string(),
        duration_ms: duration_ms(duration),
    });
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn samples_duration_ms(sample_count: usize) -> u64 {
    (sample_count as u64 * 1_000) / SAMPLE_RATE_HZ as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(index: u32, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            index,
            start_ms: u64::from(index) * 1_000,
            end_ms: u64::from(index + 1) * 1_000,
            text: text.to_string(),
            speaker: None,
        }
    }

    #[test]
    fn filters_runaway_duplicate_phrases() {
        let mut segments = vec![
            segment(0, "I also visited the museum Orsay, which I really liked."),
            segment(1, "Because its paintings had a unique atmosphere."),
        ];
        for index in 2..16 {
            segments.push(segment(index, "The house is a place to be in the room."));
        }

        let filtered = filter_hallucinated_segments(segments);

        assert_eq!(filtered.len(), 4);
        assert_eq!(
            filtered
                .iter()
                .filter(|segment| segment.text == "The house is a place to be in the room.")
                .count(),
            2
        );
        assert_eq!(filtered[2].index, 2);
        assert_eq!(filtered[3].index, 3);
    }

    #[test]
    fn caps_short_acknowledgement_runs() {
        let segments = (0..8)
            .map(|index| segment(index, "Mm-hmm."))
            .collect::<Vec<_>>();

        let filtered = filter_hallucinated_segments(segments);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|segment| segment.text == "Mm-hmm."));
    }

    #[test]
    fn keeps_distinct_dialogue_segments() {
        let segments = vec![
            segment(0, "Good morning."),
            segment(1, "I'm not bad. Thank you."),
            segment(2, "Я записал."),
            segment(3, "Yesterday I was in Berlin."),
            segment(4, "Then I saw many cars."),
        ];

        let filtered = filter_hallucinated_segments(segments.clone());

        assert_eq!(filtered.len(), segments.len());
        assert_eq!(transcript_text(&filtered), transcript_text(&segments));
    }

    #[test]
    fn drops_subtitle_and_music_hallucinations() {
        let segments = vec![
            segment(0, "Good morning."),
            segment(1, "Субтитры создавал DimaTorzok"),
            segment(2, "Музыка"),
            segment(3, "without a doubt"),
        ];

        let filtered = filter_hallucinated_segments(segments);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].text, "Good morning.");
        assert_eq!(filtered[1].text, "without a doubt");
    }

    #[test]
    fn caps_short_acknowledgements_across_long_runs() {
        let mut segments = vec![segment(0, "What have you seen during holidays?")];
        for index in 1..24 {
            let text = if index % 2 == 0 { "Угу." } else { "Mm-hmm." };
            segments.push(segment(index, text));
        }
        segments.push(segment(24, "I have been to Italy."));

        let filtered = filter_hallucinated_segments(segments);

        let acknowledgement_count = filtered
            .iter()
            .filter(|segment| {
                let key = normalize_repetition_key(&segment.text);
                key == "угу" || key == "mm hmm"
            })
            .count();

        assert_eq!(acknowledgement_count, 4);
        assert!(filtered
            .iter()
            .any(|segment| segment.text == "I have been to Italy."));
    }

    #[test]
    fn multilingual_quality_uses_short_language_switch_chunks() {
        let five_minutes = SAMPLE_RATE_HZ * 60 * 5;
        let twenty_minutes = SAMPLE_RATE_HZ * 60 * 20;

        assert_eq!(calculate_adaptive_chunk_size("multi", five_minutes), 10);
        assert_eq!(
            calculate_adaptive_chunk_size("multilingual-quality", twenty_minutes),
            10
        );
        assert_eq!(multilingual_overlap_seconds("multi"), 1);
    }

    #[test]
    fn recognizes_quality_aliases_as_chunked_multilingual() {
        for language in [
            "multi",
            "multilingual",
            "multi-quality",
            "multilingual-quality",
        ] {
            assert!(
                is_chunked_multilingual_language(language),
                "{language} should use chunked multilingual quality mode"
            );
        }
    }

    #[test]
    fn multilingual_prompt_explicitly_preserves_mixed_ru_en() {
        let prompt = build_initial_prompt_opt(Some("Sergei, LocalScribe"), true)
            .unwrap()
            .unwrap();

        assert!(prompt.contains("Русская речь -> русский текст"));
        assert!(prompt.contains("English speech -> English text"));
        assert!(prompt.contains("Не переводить"));
        assert!(prompt.contains("Do not translate"));
        assert!(prompt.contains("Sergei, LocalScribe"));
    }

    #[test]
    fn multilingual_chunk_plan_adds_overlap_without_moving_output_windows() {
        let samples = SAMPLE_RATE_HZ * 25;
        let plan = build_multilingual_chunk_plan(samples, 10, 1);

        assert_eq!(plan.len(), 3);
        assert_eq!(plan[0].decode_start, 0);
        assert_eq!(plan[0].decode_end, SAMPLE_RATE_HZ * 11);
        assert_eq!(plan[0].output_start, 0);
        assert_eq!(plan[0].output_end, SAMPLE_RATE_HZ * 10);
        assert_eq!(plan[1].decode_start, SAMPLE_RATE_HZ * 9);
        assert_eq!(plan[1].decode_end, SAMPLE_RATE_HZ * 21);
        assert_eq!(plan[1].output_start, SAMPLE_RATE_HZ * 10);
        assert_eq!(plan[1].output_end, SAMPLE_RATE_HZ * 20);
        assert_eq!(plan[2].decode_start, SAMPLE_RATE_HZ * 19);
        assert_eq!(plan[2].decode_end, SAMPLE_RATE_HZ * 25);
        assert_eq!(plan[2].output_start, SAMPLE_RATE_HZ * 20);
        assert_eq!(plan[2].output_end, SAMPLE_RATE_HZ * 25);
    }

    #[test]
    fn overlap_crop_keeps_segments_by_midpoint_and_reindexes() {
        let chunk = MultilingualChunk {
            decode_start: SAMPLE_RATE_HZ * 18,
            decode_end: SAMPLE_RATE_HZ * 42,
            output_start: SAMPLE_RATE_HZ * 20,
            output_end: SAMPLE_RATE_HZ * 40,
        };
        let segments = vec![
            TranscriptSegment {
                index: 10,
                start_ms: 18_500,
                end_ms: 19_500,
                text: "left overlap".to_string(),
                speaker: None,
            },
            TranscriptSegment {
                index: 11,
                start_ms: 19_800,
                end_ms: 20_300,
                text: "boundary text".to_string(),
                speaker: None,
            },
            TranscriptSegment {
                index: 12,
                start_ms: 39_800,
                end_ms: 40_300,
                text: "right overlap".to_string(),
                speaker: None,
            },
        ];

        let cropped = crop_segments_to_chunk_output(segments, &chunk, 4);

        assert_eq!(cropped.len(), 1);
        assert_eq!(cropped[0].index, 4);
        assert_eq!(cropped[0].text, "boundary text");
    }

    #[test]
    fn auto_engine_prefers_gpu() {
        assert!(engine_should_use_gpu(Some("auto"), false, false));
    }

    #[test]
    fn balanced_decoding_uses_fast_greedy_sampling() {
        match sampling_strategy_for_decoding(Some("balanced")) {
            SamplingStrategy::Greedy { best_of } => assert_eq!(best_of, 1),
            SamplingStrategy::BeamSearch { .. } => panic!("balanced should use greedy decoding"),
        }
    }

    #[test]
    fn accurate_decoding_uses_beam_search_sampling() {
        match sampling_strategy_for_decoding(Some("accurate")) {
            SamplingStrategy::BeamSearch {
                beam_size,
                patience,
            } => {
                assert_eq!(beam_size, 5);
                assert_eq!(patience, -1.0);
            }
            SamplingStrategy::Greedy { .. } => panic!("accurate should use beam search decoding"),
        }
    }

    #[test]
    fn metal_engine_prefers_gpu() {
        assert!(engine_should_use_gpu(Some("metal"), false, false));
    }

    #[test]
    fn cpu_engine_disables_gpu() {
        assert!(!engine_should_use_gpu(Some("cpu"), false, false));
    }

    #[test]
    fn disable_env_overrides_engine() {
        assert!(!engine_should_use_gpu(Some("metal"), true, false));
    }

    #[test]
    fn force_env_overrides_cpu_engine() {
        assert!(engine_should_use_gpu(Some("cpu"), false, true));
    }

    #[test]
    fn auto_engine_allows_cpu_fallback() {
        assert!(engine_allows_cpu_fallback(Some("auto"), false));
    }

    #[test]
    fn forced_metal_disables_cpu_fallback() {
        assert!(!engine_allows_cpu_fallback(Some("auto"), true));
    }
}
