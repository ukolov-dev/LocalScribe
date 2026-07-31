use hound::{SampleFormat, WavReader};
use localscribe_lib::{
    export::format_transcript,
    types::{ExportFormat, TranscriptSegment},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if !(4..=5).contains(&args.len()) {
        eprintln!("usage: smoke_transcribe <input-media> <model-bin> <output-dir> [language]");
        std::process::exit(2);
    }

    let input_path = PathBuf::from(&args[1]);
    let model_path = PathBuf::from(&args[2]);
    let output_dir = PathBuf::from(&args[3]);
    let language = args.get(4).map(String::as_str);
    fs::create_dir_all(&output_dir)?;

    let wav_path = output_dir.join("smoke-input.wav");
    convert_to_wav(&input_path, &wav_path)?;
    let samples = read_wav_as_f32(&wav_path)?;
    let segments = transcribe(&model_path, &samples, language)?;
    let text = segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(
        output_dir.join("transcript.txt"),
        format_transcript(ExportFormat::Txt, &text, &segments),
    )?;
    fs::write(
        output_dir.join("transcript.md"),
        format_transcript(ExportFormat::Md, &text, &segments),
    )?;

    println!(
        "Smoke transcription complete: {} segments, {} chars",
        segments.len(),
        text.chars().count()
    );
    Ok(())
}

fn convert_to_wav(input_path: &Path, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let ffmpeg = resolve_ffmpeg();
    let output = Command::new(ffmpeg)
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(input_path)
        .arg("-vn")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(output_path)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()
            .into())
    }
}

fn resolve_ffmpeg() -> PathBuf {
    if let Some(path) = env::var_os("LOCALSCRIBE_FFMPEG").map(PathBuf::from) {
        return path;
    }

    let sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("ffmpeg-aarch64-apple-darwin");
    if sidecar.is_file() {
        return sidecar;
    }

    PathBuf::from("ffmpeg")
}

fn read_wav_as_f32(path: &Path) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != 16_000 {
        return Err("converted WAV must be 16 kHz mono".into());
    }

    match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => Ok(reader
            .samples::<i16>()
            .map(|sample| sample.map(|value| value as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()?),
        (SampleFormat::Float, 32) => Ok(reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?),
        _ => Err("unsupported converted WAV sample format".into()),
    }
}

fn transcribe(
    model_path: &Path,
    samples: &[f32],
    language: Option<&str>,
) -> Result<Vec<TranscriptSegment>, Box<dyn std::error::Error>> {
    let mut context_params = WhisperContextParameters::default();
    context_params.use_gpu = env::var("LOCALSCRIBE_SMOKE_USE_GPU").as_deref() == Ok("1");
    context_params.gpu_device = 0;

    let context = WhisperContext::new_with_params(model_path, context_params)?;
    let mut state = context.create_state()?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_n_threads(
        std::thread::available_parallelism()
            .map(|threads| threads.get() as i32)
            .unwrap_or(4),
    );
    params.set_language(language);
    params.set_debug_mode(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_no_timestamps(false);

    state.full(params, samples)?;

    let mut segments = Vec::new();
    for index in 0..state.full_n_segments() {
        if let Some(segment) = state.get_segment(index) {
            segments.push(TranscriptSegment {
                index: index as u32,
                start_ms: segment.start_timestamp().max(0) as u64 * 10,
                end_ms: segment.end_timestamp().max(0) as u64 * 10,
                text: segment.to_str_lossy()?.trim().to_string(),
                speaker: None,
            });
        }
    }

    Ok(segments)
}
