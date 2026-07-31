use crate::{
    errors::{display_path, invalid, AppError, AppResult},
    settings,
    types::{SelectCustomModelRequest, SpeakerModelFileInfo, SpeakerModelInfo},
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy)]
pub struct SpeakerModelFileSpec {
    pub id: &'static str,
    pub file_name: &'static str,
    pub size_label: &'static str,
    pub size_bytes: u64,
    pub min_bytes: u64,
    pub url: &'static str,
    pub sha256: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
pub struct SpeakerModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub size_label: &'static str,
    pub size_bytes: u64,
    pub recommended: bool,
    pub quality_label: &'static str,
    pub runtime_label: &'static str,
    pub files: &'static [SpeakerModelFileSpec],
}

pub const CUSTOM_SPEAKER_MODEL_ID: &str = "custom-speaker";

const SILERO_VAD: SpeakerModelFileSpec = SpeakerModelFileSpec {
    id: "silero-vad",
    file_name: "silero_vad.onnx",
    size_label: "~2.2 MB",
    size_bytes: 2_327_524,
    min_bytes: 1_500_000,
    url: "https://huggingface.co/deepghs/silero-vad-onnx/resolve/main/silero_vad.onnx",
    sha256: Some("2623a2953f6ff3d2c1e61740c6cdb7168133479b267dfef114a4a3cc5bdd788f"),
};

const WESPEAKER_RESNET34: SpeakerModelFileSpec = SpeakerModelFileSpec {
    id: "wespeaker-resnet34-lm",
    file_name: "voxceleb_resnet34_LM.onnx",
    size_label: "~26.5 MB",
    size_bytes: 26_530_309,
    min_bytes: 24 * 1024 * 1024,
    url: "https://huggingface.co/Wespeaker/wespeaker-voxceleb-resnet34-LM/resolve/main/voxceleb_resnet34_LM.onnx",
    sha256: Some("7bb2f06e9df17cdf1ef14ee8a15ab08ed28e8d0ef5054ee135741560df2ec068"),
};

const SPEECHBRAIN_ECAPA: SpeakerModelFileSpec = SpeakerModelFileSpec {
    id: "speechbrain-ecapa",
    file_name: "speechbrain-ecapa-embedding_model.ckpt",
    size_label: "~83.3 MB",
    size_bytes: 83_316_686,
    min_bytes: 75 * 1024 * 1024,
    url:
        "https://huggingface.co/speechbrain/spkrec-ecapa-voxceleb/resolve/main/embedding_model.ckpt",
    sha256: Some("0575cb64845e6b9a10db9bcb74d5ac32b326b8dc90352671d345e2ee3d0126a2"),
};

const TITANET_LARGE: SpeakerModelFileSpec = SpeakerModelFileSpec {
    id: "nvidia-titanet-large",
    file_name: "speakerverification_en_titanet_large.nemo",
    size_label: "~94.8 MB",
    size_bytes: 101_621_760,
    min_bytes: 85 * 1024 * 1024,
    url: "https://huggingface.co/nvidia/speakerverification_en_titanet_large/resolve/main/speakerverification_en_titanet_large.nemo",
    sha256: Some("e838520693f269e7984f55bc8eb3c2d60ccf246bf4b896d4be9bcabe3e4b0fe3"),
};

const SPEAKER_LITE_FILES: &[SpeakerModelFileSpec] = &[SILERO_VAD, WESPEAKER_RESNET34];
const SPEAKER_ECAPA_FILES: &[SpeakerModelFileSpec] = &[SILERO_VAD, SPEECHBRAIN_ECAPA];
const SPEAKER_TITANET_FILES: &[SpeakerModelFileSpec] = &[SILERO_VAD, TITANET_LARGE];

pub const SPEAKER_MODEL_SPECS: &[SpeakerModelSpec] = &[
    SpeakerModelSpec {
        id: "speaker-lite",
        name: "Speaker Lite",
        size_label: "~29 MB",
        size_bytes: 28_857_833,
        recommended: true,
        quality_label: "Recommended",
        runtime_label: "ONNX VAD + ONNX embeddings",
        files: SPEAKER_LITE_FILES,
    },
    SpeakerModelSpec {
        id: "speaker-ecapa",
        name: "ECAPA-TDNN",
        size_label: "~86 MB",
        size_bytes: 85_644_210,
        recommended: false,
        quality_label: "Better quality",
        runtime_label: "Checkpoint, adapter required",
        files: SPEAKER_ECAPA_FILES,
    },
    SpeakerModelSpec {
        id: "speaker-titanet",
        name: "TitaNet Large",
        size_label: "~97 MB",
        size_bytes: 103_949_284,
        recommended: false,
        quality_label: "Better quality",
        runtime_label: "NeMo archive, adapter required",
        files: SPEAKER_TITANET_FILES,
    },
];

pub fn speaker_models_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Config(error.to_string()))?
        .join("models")
        .join("diarization");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn spec_by_id(model_id: &str) -> AppResult<&'static SpeakerModelSpec> {
    SPEAKER_MODEL_SPECS
        .iter()
        .find(|spec| spec.id == model_id)
        .ok_or_else(|| AppError::UnknownModel(model_id.to_string()))
}

pub fn path_for_file_spec(
    app: &AppHandle,
    pack: &SpeakerModelSpec,
    file: &SpeakerModelFileSpec,
) -> AppResult<PathBuf> {
    Ok(speaker_models_dir(app)?.join(pack.id).join(file.file_name))
}

pub fn part_path_for_file_spec(
    app: &AppHandle,
    pack: &SpeakerModelSpec,
    file: &SpeakerModelFileSpec,
) -> AppResult<PathBuf> {
    Ok(speaker_models_dir(app)?
        .join(pack.id)
        .join(format!("{}.part", file.file_name)))
}

pub fn list_speaker_models(app: &AppHandle) -> AppResult<Vec<SpeakerModelInfo>> {
    let mut models = SPEAKER_MODEL_SPECS
        .iter()
        .map(|spec| speaker_model_info_for_spec(app, spec))
        .collect::<AppResult<Vec<_>>>()?;

    let stored_settings = settings::load(app)?;
    models.push(custom_speaker_model_info(
        stored_settings.custom_speaker_model_path.as_deref(),
    )?);
    Ok(models)
}

pub fn speaker_model_info_for_spec(
    app: &AppHandle,
    spec: &SpeakerModelSpec,
) -> AppResult<SpeakerModelInfo> {
    let files = spec
        .files
        .iter()
        .map(|file| speaker_file_info_for_spec(app, spec, file))
        .collect::<AppResult<Vec<_>>>()?;
    let installed = files.iter().all(|file| file.installed);

    Ok(SpeakerModelInfo {
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        size_label: spec.size_label.to_string(),
        size_bytes: Some(spec.size_bytes),
        recommended: spec.recommended,
        quality_label: spec.quality_label.to_string(),
        runtime_label: spec.runtime_label.to_string(),
        installed,
        custom: false,
        files,
    })
}

pub fn speaker_file_info_for_spec(
    app: &AppHandle,
    pack: &SpeakerModelSpec,
    file: &SpeakerModelFileSpec,
) -> AppResult<SpeakerModelFileInfo> {
    let path = path_for_file_spec(app, pack, file)?;
    let installed = valid_model_file(&path, file.min_bytes);

    Ok(SpeakerModelFileInfo {
        id: file.id.to_string(),
        file_name: file.file_name.to_string(),
        path: installed.then(|| display_path(&path)),
        installed,
        size_label: file.size_label.to_string(),
        size_bytes: Some(file.size_bytes),
    })
}

pub fn custom_speaker_model_info(path: Option<&str>) -> AppResult<SpeakerModelInfo> {
    let path_buf = path.map(PathBuf::from);
    let installed = path_buf.as_ref().is_some_and(|path| path.is_file());
    let size_label = path_buf
        .as_ref()
        .and_then(|path| fs::metadata(path).ok())
        .map(|metadata| format_bytes(metadata.len()))
        .unwrap_or_else(|| "-".to_string());
    let file_name = path_buf
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "Custom ONNX embedding model".to_string());
    let size_bytes = path_buf
        .as_ref()
        .and_then(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len());

    Ok(SpeakerModelInfo {
        id: CUSTOM_SPEAKER_MODEL_ID.to_string(),
        name: "Custom speaker ONNX".to_string(),
        size_label: size_label.clone(),
        size_bytes,
        recommended: false,
        quality_label: "Manual".to_string(),
        runtime_label: "Custom ONNX embedding model".to_string(),
        installed,
        custom: true,
        files: vec![SpeakerModelFileInfo {
            id: "custom-speaker-embedding".to_string(),
            file_name,
            path: installed.then(|| display_path(path_buf.as_ref().expect("checked above"))),
            installed,
            size_label,
            size_bytes,
        }],
    })
}

pub fn select_custom_speaker_model(
    app: &AppHandle,
    request: SelectCustomModelRequest,
) -> AppResult<SpeakerModelInfo> {
    let path = PathBuf::from(request.path);
    validate_custom_speaker_model_path(&path)?;

    let mut stored_settings = settings::load(app)?;
    stored_settings.custom_speaker_model_path = Some(display_path(&path));
    stored_settings.default_speaker_model_id = Some(CUSTOM_SPEAKER_MODEL_ID.to_string());
    settings::save(app, &stored_settings)?;

    custom_speaker_model_info(stored_settings.custom_speaker_model_path.as_deref())
}

pub fn delete_speaker_model(app: &AppHandle, model_id: &str) -> AppResult<Vec<SpeakerModelInfo>> {
    let mut stored_settings = settings::load(app)?;

    if model_id == CUSTOM_SPEAKER_MODEL_ID {
        stored_settings.custom_speaker_model_path = None;
        if stored_settings.default_speaker_model_id.as_deref() == Some(CUSTOM_SPEAKER_MODEL_ID) {
            stored_settings.default_speaker_model_id = None;
        }
        settings::save(app, &stored_settings)?;
        return list_speaker_models(app);
    }

    let spec = spec_by_id(model_id)?;
    let dir = speaker_models_dir(app)?.join(spec.id);
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }

    if stored_settings.default_speaker_model_id.as_deref() == Some(model_id) {
        stored_settings.default_speaker_model_id = None;
        settings::save(app, &stored_settings)?;
    }

    list_speaker_models(app)
}

pub fn set_default_speaker_model_if_empty(app: &AppHandle, model_id: &str) -> AppResult<()> {
    let mut stored_settings = settings::load(app)?;
    if stored_settings.default_speaker_model_id.is_none() {
        stored_settings.default_speaker_model_id = Some(model_id.to_string());
        settings::save(app, &stored_settings)?;
    }
    Ok(())
}

pub fn validate_downloaded_speaker_file(path: &Path, file: &SpeakerModelFileSpec) -> AppResult<()> {
    let size = fs::metadata(path)?.len();
    if size < file.min_bytes {
        return Err(AppError::CorruptModel);
    }
    Ok(())
}

fn validate_custom_speaker_model_path(path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(invalid("Choose an existing ONNX speaker model file."));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("onnx") {
        return Err(invalid("Custom speaker model must be an .onnx file."));
    }
    Ok(())
}

fn valid_model_file(path: &Path, min_bytes: u64) -> bool {
    path.is_file()
        && fs::metadata(path)
            .map(|metadata| metadata.len() >= min_bytes)
            .unwrap_or(false)
}

fn format_bytes(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GB {
        format!("{:.1} GB", bytes / GB)
    } else {
        format!("{:.0} MB", bytes / MB)
    }
}
