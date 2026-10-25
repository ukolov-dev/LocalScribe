use crate::errors::AppResult;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::AppHandle;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub default_model_id: Option<String>,
    pub custom_model_path: Option<String>,
    pub custom_speaker_model_path: Option<String>,
}

fn settings_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = crate::storage::data_dir(app)?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> AppResult<AppSettings> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> AppResult<()> {
    let path = settings_path(app)?;
    let raw = serde_json::to_string_pretty(settings)?;
    fs::write(path, raw)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_keep_custom_files_without_the_unused_speaker_default() {
        let settings: AppSettings = serde_json::from_str(
            r#"{
            "defaultModelId": "custom",
            "customSpeakerModelPath": "/tmp/custom.onnx",
            "defaultSpeakerModelId": "speaker-lite"
        }"#,
        )
        .unwrap();
        assert_eq!(settings.default_model_id.as_deref(), Some("custom"));
        assert_eq!(
            settings.custom_speaker_model_path.as_deref(),
            Some("/tmp/custom.onnx")
        );
        assert!(!serde_json::to_string(&settings)
            .unwrap()
            .contains("defaultSpeakerModelId"));
    }
}
