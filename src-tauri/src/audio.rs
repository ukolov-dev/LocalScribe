use crate::errors::{display_path, AppError, AppResult};
use hound::{SampleFormat, WavReader};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Manager};

const SUPPORTED_EXTENSIONS: &[&str] = &["aac", "mp3", "wav", "m4a", "mp4", "mov", "webm"];

pub struct PreparedAudio {
    pub samples: Vec<f32>,
    pub wav_path: PathBuf,
}

pub fn prepare_audio(app: &AppHandle, job_id: &str, input_path: &Path) -> AppResult<PreparedAudio> {
    validate_input_file(input_path)?;
    let wav_path = temp_wav_path(app, job_id)?;
    convert_to_wav(app, input_path, &wav_path)?;
    let samples = read_wav_as_f32(&wav_path)?;

    if samples.is_empty() {
        cleanup_temp_file(&wav_path);
        return Err(AppError::Audio(
            "Converted audio contains no samples.".to_string(),
        ));
    }

    Ok(PreparedAudio { samples, wav_path })
}

pub fn cleanup_temp_file(path: &Path) {
    let _ = fs::remove_file(path);
}

fn validate_input_file(path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(AppError::InvalidInput(format!(
            "Input file does not exist: {}",
            display_path(path)
        )));
    }

    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or(AppError::UnsupportedFormat)?;

    if SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        Ok(())
    } else {
        Err(AppError::UnsupportedFormat)
    }
}

fn temp_wav_path(app: &AppHandle, job_id: &str) -> AppResult<PathBuf> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| AppError::Config(error.to_string()))?
        .join("temp");
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join(format!("{job_id}.wav")))
}

fn convert_to_wav(app: &AppHandle, input_path: &Path, output_path: &Path) -> AppResult<()> {
    let ffmpeg = resolve_ffmpeg(app)?;
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
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::MissingFfmpeg
            } else {
                AppError::Audio(error.to_string())
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::Audio(if stderr.is_empty() {
            "ffmpeg failed to convert the input file.".to_string()
        } else {
            stderr
        }));
    }

    Ok(())
}

fn resolve_ffmpeg(app: &AppHandle) -> AppResult<PathBuf> {
    if let Some(path) = env::var_os("LOCALSCRIBE_FFMPEG").map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
    }

    for candidate in sidecar_candidates(app) {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    if command_exists("ffmpeg") {
        return Ok(PathBuf::from("ffmpeg"));
    }

    Err(AppError::MissingFfmpeg)
}

fn sidecar_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("ffmpeg"));
        candidates.push(resource_dir.join("ffmpeg-aarch64-apple-darwin"));
        candidates.push(resource_dir.join("binaries").join("ffmpeg"));
        candidates.push(
            resource_dir
                .join("binaries")
                .join("ffmpeg-aarch64-apple-darwin"),
        );
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("ffmpeg"));
            candidates.push(exe_dir.join("ffmpeg-aarch64-apple-darwin"));
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    candidates.push(
        manifest_dir
            .join("binaries")
            .join("ffmpeg-aarch64-apple-darwin"),
    );
    candidates.push(manifest_dir.join("binaries").join("ffmpeg"));

    candidates
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("-version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn read_wav_as_f32(path: &Path) -> AppResult<Vec<f32>> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();

    if spec.channels != 1 || spec.sample_rate != 16_000 {
        return Err(AppError::Audio(
            "Converted WAV must be 16 kHz mono.".to_string(),
        ));
    }

    match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|sample| sample.map(|value| value as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from),
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::from),
        _ => Err(AppError::Audio(format!(
            "Unsupported converted WAV format: {:?}, {} bits.",
            spec.sample_format, spec.bits_per_sample
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_input_file;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn accepts_standalone_aac_files() {
        let path = std::env::temp_dir().join(format!("localscribe-{}.AAC", Uuid::new_v4()));
        fs::write(&path, b"synthetic test fixture").expect("create AAC input");

        let result = validate_input_file(&path);
        fs::remove_file(&path).expect("remove AAC input");

        assert!(result.is_ok());
    }
}
