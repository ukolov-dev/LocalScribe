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

impl Drop for PreparedAudio {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.wav_path);
    }
}

pub fn prepare_audio(app: &AppHandle, input_path: &Path) -> AppResult<PreparedAudio> {
    validate_input_file(input_path)?;
    let ffmpeg = resolve_ffmpeg(app)?;
    prepare_wav(&ffmpeg, input_path, temp_wav_path(app)?)
}

fn prepare_wav(ffmpeg: &Path, input_path: &Path, wav_path: PathBuf) -> AppResult<PreparedAudio> {
    let mut prepared = PreparedAudio {
        samples: Vec::new(),
        wav_path,
    };
    convert_to_wav(ffmpeg, input_path, &prepared.wav_path)?;
    prepared.samples = read_wav_as_f32(&prepared.wav_path)?;

    if prepared.samples.is_empty() {
        return Err(AppError::Audio(
            "Converted audio contains no samples.".to_string(),
        ));
    }

    Ok(prepared)
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

fn temp_wav_path(app: &AppHandle) -> AppResult<PathBuf> {
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| AppError::Config(error.to_string()))?
        .join("temp");
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join(format!("{}.wav", uuid::Uuid::new_v4())))
}

fn convert_to_wav(ffmpeg: &Path, input_path: &Path, output_path: &Path) -> AppResult<()> {
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
    use super::{prepare_wav, validate_input_file};
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
    #[cfg(unix)]
    #[test]
    fn removes_partial_wav_on_conversion_and_read_errors() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("localscribe-audio-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let ffmpeg = dir.join("ffmpeg");
        for exit_code in [0, 1] {
            fs::write(&ffmpeg, format!("#!/bin/sh\nfor output do :; done\nprintf 'invalid WAV' > \"$output\"\nexit {exit_code}\n")).unwrap();
            fs::set_permissions(&ffmpeg, fs::Permissions::from_mode(0o700)).unwrap();
            let wav = dir.join(format!("partial-{exit_code}.wav"));
            assert!(prepare_wav(&ffmpeg, &dir.join("input.mp4"), wav.clone()).is_err());
            assert!(!wav.exists(), "failed preparation left a WAV behind");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn prepared_audio_owns_the_wav_until_it_is_dropped() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("localscribe-audio-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.wav");
        let mut writer = hound::WavWriter::create(
            &input,
            hound::WavSpec {
                channels: 1,
                sample_rate: 16_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        writer.write_sample(100_i16).unwrap();
        writer.finalize().unwrap();
        let ffmpeg = dir.join("ffmpeg");
        fs::write(&ffmpeg, "#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = '-i' ]; then shift; input=$1; fi\n  output=$1\n  shift\ndone\ncp \"$input\" \"$output\"\n").unwrap();
        fs::set_permissions(&ffmpeg, fs::Permissions::from_mode(0o700)).unwrap();
        let wav = dir.join("prepared.wav");
        let prepared = prepare_wav(&ffmpeg, &input, wav.clone()).unwrap();
        assert_eq!(prepared.samples.len(), 1);
        assert!(wav.is_file());
        drop(prepared);
        assert!(!wav.exists());
        assert!(input.is_file());
        fs::remove_dir_all(dir).unwrap();
    }
}
