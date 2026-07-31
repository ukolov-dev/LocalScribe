use serde::Serialize;
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Model is not installed. Download a model or choose a custom .bin file.")]
    NoModel,
    #[error("Unknown model id: {0}")]
    UnknownModel(String),
    #[error("Unsupported file format. Supported formats: mp3, wav, m4a, mp4, mov, webm.")]
    UnsupportedFormat,
    #[error("ffmpeg is not available. Install ffmpeg or bundle it as a sidecar for release.")]
    MissingFfmpeg,
    #[error("The model file looks incomplete or corrupted. Delete it and download again.")]
    CorruptModel,
    #[error("Transcription was cancelled.")]
    Cancelled,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Filesystem error: {0}")]
    Io(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Audio conversion failed: {0}")]
    Audio(String),
    #[error("Whisper engine failed: {0}")]
    Whisper(String),
    #[error("Transcription completed but produced no text. Try selecting the language manually or use the Q5 model.")]
    EmptyTranscript,
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializableError {
    code: &'static str,
    message: String,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoModel => "NO_MODEL",
            Self::UnknownModel(_) => "UNKNOWN_MODEL",
            Self::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            Self::MissingFfmpeg => "MISSING_FFMPEG",
            Self::CorruptModel => "CORRUPT_MODEL",
            Self::Cancelled => "CANCELLED",
            Self::InvalidInput(_) => "INVALID_INPUT",
            Self::Io(_) => "IO_ERROR",
            Self::Network(_) => "NETWORK_ERROR",
            Self::Audio(_) => "AUDIO_ERROR",
            Self::Whisper(_) => "WHISPER_ERROR",
            Self::EmptyTranscript => "EMPTY_TRANSCRIPT",
            Self::Config(_) => "CONFIG_ERROR",
            Self::Unexpected(_) => "UNEXPECTED_ERROR",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerializableError {
            code: self.code(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        Self::Network(error.to_string())
    }
}

impl From<hound::Error> for AppError {
    fn from(error: hound::Error) -> Self {
        Self::Audio(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::Config(error.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(error: tauri::Error) -> Self {
        Self::Unexpected(error.to_string())
    }
}

impl From<whisper_rs::WhisperError> for AppError {
    fn from(error: whisper_rs::WhisperError) -> Self {
        Self::Whisper(error.to_string())
    }
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn display_path(path: &std::path::Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn invalid<T: fmt::Display>(message: T) -> AppError {
    AppError::InvalidInput(message.to_string())
}
