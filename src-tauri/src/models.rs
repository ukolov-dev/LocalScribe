use crate::{
    errors::{display_path, invalid, AppError, AppResult},
    settings,
    types::{ModelInfo, SelectCustomModelRequest, TranscribeOptions},
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Copy)]
pub struct ModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub file_name: &'static str,
    pub size_label: &'static str,
    pub size_bytes: u64,
    pub min_bytes: u64,
    pub recommended: bool,
    pub quality_label: &'static str,
    pub url: &'static str,
    pub sha256: Option<&'static str>,
}

pub const CUSTOM_MODEL_ID: &str = "custom";

pub const MODEL_SPECS: &[ModelSpec] = &[
    ModelSpec {
        id: "large-v3-turbo-q5_0",
        name: "Turbo Q5",
        file_name: "ggml-large-v3-turbo-q5_0.bin",
        size_label: "~547 MB",
        size_bytes: 574_041_195,
        min_bytes: 500 * 1024 * 1024,
        recommended: true,
        quality_label: "Recommended",
        url:
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin",
        sha256: Some("394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2"),
    },
    ModelSpec {
        id: "large-v3-turbo",
        name: "Turbo Full",
        file_name: "ggml-large-v3-turbo.bin",
        size_label: "~1.5 GB",
        size_bytes: 1_624_555_275,
        min_bytes: 1_300 * 1024 * 1024,
        recommended: false,
        quality_label: "Best quality",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        sha256: Some("1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69"),
    },
];

pub fn models_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| AppError::Config(error.to_string()))?
        .join("models");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn spec_by_id(model_id: &str) -> AppResult<&'static ModelSpec> {
    MODEL_SPECS
        .iter()
        .find(|spec| spec.id == model_id)
        .ok_or_else(|| AppError::UnknownModel(model_id.to_string()))
}

pub fn path_for_spec(app: &AppHandle, spec: &ModelSpec) -> AppResult<PathBuf> {
    Ok(models_dir(app)?.join(spec.file_name))
}

pub fn part_path_for_spec(app: &AppHandle, spec: &ModelSpec) -> AppResult<PathBuf> {
    Ok(models_dir(app)?.join(format!("{}.part", spec.file_name)))
}

pub fn list_models(app: &AppHandle) -> AppResult<Vec<ModelInfo>> {
    let mut models = MODEL_SPECS
        .iter()
        .map(|spec| model_info_for_spec(app, spec))
        .collect::<AppResult<Vec<_>>>()?;

    let stored_settings = settings::load(app)?;
    models.push(custom_model_info(
        stored_settings.custom_model_path.as_deref(),
    )?);
    Ok(models)
}

pub fn model_info_for_spec(app: &AppHandle, spec: &ModelSpec) -> AppResult<ModelInfo> {
    let path = path_for_spec(app, spec)?;
    let installed = valid_model_file(&path, spec.min_bytes);

    Ok(ModelInfo {
        id: spec.id.to_string(),
        name: spec.name.to_string(),
        file_name: Some(spec.file_name.to_string()),
        size_label: spec.size_label.to_string(),
        size_bytes: Some(spec.size_bytes),
        recommended: spec.recommended,
        quality_label: spec.quality_label.to_string(),
        installed,
        path: installed.then(|| display_path(&path)),
        custom: false,
    })
}

pub fn custom_model_info(path: Option<&str>) -> AppResult<ModelInfo> {
    let path_buf = path.map(PathBuf::from);
    let installed = path_buf.as_ref().is_some_and(|path| path.is_file());
    let size_label = path_buf
        .as_ref()
        .and_then(|path| fs::metadata(path).ok())
        .map(|metadata| format_bytes(metadata.len()))
        .unwrap_or_else(|| "-".to_string());

    Ok(ModelInfo {
        id: CUSTOM_MODEL_ID.to_string(),
        name: "Custom model file".to_string(),
        file_name: path_buf
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().to_string()),
        size_label,
        size_bytes: path_buf
            .as_ref()
            .and_then(|path| fs::metadata(path).ok())
            .map(|metadata| metadata.len()),
        recommended: false,
        quality_label: "Manual".to_string(),
        installed,
        path: installed.then(|| display_path(path_buf.as_ref().expect("checked above"))),
        custom: true,
    })
}

pub fn select_custom_model(
    app: &AppHandle,
    request: SelectCustomModelRequest,
) -> AppResult<ModelInfo> {
    let path = PathBuf::from(request.path);
    validate_custom_model_path(&path)?;

    let mut stored_settings = settings::load(app)?;
    stored_settings.custom_model_path = Some(display_path(&path));
    stored_settings.default_model_id = Some(CUSTOM_MODEL_ID.to_string());
    settings::save(app, &stored_settings)?;

    custom_model_info(stored_settings.custom_model_path.as_deref())
}

pub fn delete_model(app: &AppHandle, model_id: &str) -> AppResult<Vec<ModelInfo>> {
    let mut stored_settings = settings::load(app)?;

    if model_id == CUSTOM_MODEL_ID {
        stored_settings.custom_model_path = None;
        if stored_settings.default_model_id.as_deref() == Some(CUSTOM_MODEL_ID) {
            stored_settings.default_model_id = None;
        }
        settings::save(app, &stored_settings)?;
        return list_models(app);
    }

    let spec = spec_by_id(model_id)?;
    let path = path_for_spec(app, spec)?;
    let part_path = part_path_for_spec(app, spec)?;

    if path.exists() {
        fs::remove_file(path)?;
    }
    if part_path.exists() {
        fs::remove_file(part_path)?;
    }

    if stored_settings.default_model_id.as_deref() == Some(model_id) {
        stored_settings.default_model_id = None;
        settings::save(app, &stored_settings)?;
    }

    list_models(app)
}

pub fn resolve_model_path(app: &AppHandle, options: &TranscribeOptions) -> AppResult<PathBuf> {
    if let Some(path) = options
        .model_path
        .as_deref()
        .filter(|path| !path.is_empty())
    {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(AppError::NoModel);
    }

    if let Some(model_id) = options.model_id.as_deref().filter(|id| !id.is_empty()) {
        return resolve_model_id(app, model_id);
    }

    let stored_settings = settings::load(app)?;
    if let Some(model_id) = stored_settings.default_model_id.as_deref() {
        if let Ok(path) = resolve_model_id(app, model_id) {
            return Ok(path);
        }
    }

    MODEL_SPECS
        .iter()
        .filter_map(|spec| {
            let path = path_for_spec(app, spec).ok()?;
            valid_model_file(&path, spec.min_bytes).then_some(path)
        })
        .next()
        .ok_or(AppError::NoModel)
}

pub fn resolve_model_id(app: &AppHandle, model_id: &str) -> AppResult<PathBuf> {
    if model_id == CUSTOM_MODEL_ID {
        let stored_settings = settings::load(app)?;
        let path = stored_settings
            .custom_model_path
            .as_deref()
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .ok_or(AppError::NoModel)?;
        return Ok(path);
    }

    let spec = spec_by_id(model_id)?;
    let path = path_for_spec(app, spec)?;
    if valid_model_file(&path, spec.min_bytes) {
        Ok(path)
    } else {
        Err(AppError::NoModel)
    }
}

pub fn set_default_model_if_empty(app: &AppHandle, model_id: &str) -> AppResult<()> {
    let mut stored_settings = settings::load(app)?;
    if stored_settings.default_model_id.is_none() {
        stored_settings.default_model_id = Some(model_id.to_string());
        settings::save(app, &stored_settings)?;
    }
    Ok(())
}

pub fn validate_downloaded_model(path: &Path, spec: &ModelSpec) -> AppResult<()> {
    let size = fs::metadata(path)?.len();
    if size < spec.min_bytes {
        return Err(AppError::CorruptModel);
    }
    Ok(())
}

fn validate_custom_model_path(path: &Path) -> AppResult<()> {
    if !path.is_file() {
        return Err(invalid("Choose an existing .bin model file."));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("bin") {
        return Err(invalid("Custom model must be a .bin file."));
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
