use crate::{
    errors::{AppError, AppResult},
    models, speaker_models,
    types::{
        DownloadProgress, ModelDownloadCompleted, ModelInfo, SpeakerModelDownloadCompleted,
        SpeakerModelInfo,
    },
};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::{fs::File, io::AsyncWriteExt};

pub async fn download_model(app: AppHandle, model_id: String) -> AppResult<ModelInfo> {
    let spec = models::spec_by_id(&model_id)?;
    let final_path = models::path_for_spec(&app, spec)?;
    let part_path = models::part_path_for_spec(&app, spec)?;

    if final_path.exists() {
        models::validate_downloaded_model(&final_path, spec)?;
        let model = models::model_info_for_spec(&app, spec)?;
        app.emit(
            "model-download-completed",
            ModelDownloadCompleted {
                model: model.clone(),
            },
        )?;
        return Ok(model);
    }

    if part_path.exists() {
        tokio::fs::remove_file(&part_path).await?;
    }

    let response = reqwest::Client::new()
        .get(spec.url)
        .send()
        .await?
        .error_for_status()?;

    let total_bytes = response.content_length().or(Some(spec.size_bytes));
    let mut file = File::create(&part_path).await?;
    let mut downloaded_bytes = 0_u64;
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();

    emit_progress(&app, spec.id, 0, total_bytes)?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        downloaded_bytes += chunk.len() as u64;
        emit_progress(&app, spec.id, downloaded_bytes, total_bytes)?;
    }

    file.flush().await?;
    drop(file);

    models::validate_downloaded_model(&part_path, spec)?;
    let digest = hasher.finalize();
    let actual_sha256 = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    if let Some(expected_sha256) = spec.sha256 {
        if expected_sha256 != actual_sha256 {
            let _ = tokio::fs::remove_file(&part_path).await;
            return Err(AppError::CorruptModel);
        }
    }

    tokio::fs::rename(&part_path, &final_path).await?;
    models::set_default_model_if_empty(&app, spec.id)?;

    let model = models::model_info_for_spec(&app, spec)?;
    app.emit(
        "model-download-completed",
        ModelDownloadCompleted {
            model: model.clone(),
        },
    )?;

    Ok(model)
}

pub async fn download_speaker_model(
    app: AppHandle,
    model_id: String,
) -> AppResult<SpeakerModelInfo> {
    // Automatic first-use setup and the model manager share the same .part files.
    static DOWNLOAD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _download = DOWNLOAD.lock().await;
    let spec = speaker_models::spec_by_id(&model_id)?;
    let final_paths = spec
        .files
        .iter()
        .map(|file| speaker_models::path_for_file_spec(&app, spec, file).map(|path| (file, path)))
        .collect::<AppResult<Vec<_>>>()?;

    if final_paths.iter().all(|(file, path)| {
        path.exists() && speaker_models::validate_downloaded_speaker_file(path, file).is_ok()
    }) {
        let model = speaker_models::speaker_model_info_for_spec(&app, spec)?;
        app.emit(
            "speaker-model-download-completed",
            SpeakerModelDownloadCompleted {
                model: model.clone(),
            },
        )?;
        return Ok(model);
    }

    let total_bytes = Some(spec.size_bytes);
    let mut completed_bytes = 0_u64;
    emit_speaker_progress(&app, spec.id, 0, total_bytes)?;

    for (file, final_path) in final_paths {
        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let part_path = speaker_models::part_path_for_file_spec(&app, spec, file)?;
        if part_path.exists() {
            tokio::fs::remove_file(&part_path).await?;
        }
        if final_path.exists()
            && speaker_models::validate_downloaded_speaker_file(&final_path, file).is_ok()
        {
            completed_bytes = completed_bytes.saturating_add(file.size_bytes);
            emit_speaker_progress(&app, spec.id, completed_bytes, total_bytes)?;
            continue;
        }
        if final_path.exists() {
            tokio::fs::remove_file(&final_path).await?;
        }

        let response = reqwest::Client::new()
            .get(file.url)
            .send()
            .await?
            .error_for_status()?;

        let mut output = File::create(&part_path).await?;
        let mut downloaded_for_file = 0_u64;
        let mut hasher = Sha256::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            output.write_all(&chunk).await?;
            hasher.update(&chunk);
            downloaded_for_file += chunk.len() as u64;
            emit_speaker_progress(
                &app,
                spec.id,
                completed_bytes.saturating_add(downloaded_for_file),
                total_bytes,
            )?;
        }

        output.flush().await?;
        drop(output);

        speaker_models::validate_downloaded_speaker_file(&part_path, file)?;
        let digest = hasher.finalize();
        let actual_sha256 = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        if let Some(expected_sha256) = file.sha256 {
            if expected_sha256 != actual_sha256 {
                let _ = tokio::fs::remove_file(&part_path).await;
                return Err(AppError::CorruptModel);
            }
        }

        tokio::fs::rename(&part_path, &final_path).await?;
        completed_bytes = completed_bytes.saturating_add(file.size_bytes);
        emit_speaker_progress(&app, spec.id, completed_bytes, total_bytes)?;
    }

    let model = speaker_models::speaker_model_info_for_spec(&app, spec)?;
    app.emit(
        "speaker-model-download-completed",
        SpeakerModelDownloadCompleted {
            model: model.clone(),
        },
    )?;
    Ok(model)
}

fn emit_progress(
    app: &AppHandle,
    model_id: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) -> AppResult<()> {
    let progress = total_bytes
        .filter(|total| *total > 0)
        .map(|total| (downloaded_bytes as f32 / total as f32 * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);

    app.emit(
        "model-download-progress",
        DownloadProgress {
            model_id: model_id.to_string(),
            downloaded_bytes,
            total_bytes,
            progress,
        },
    )?;
    Ok(())
}

fn emit_speaker_progress(
    app: &AppHandle,
    model_id: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
) -> AppResult<()> {
    let progress = total_bytes
        .filter(|total| *total > 0)
        .map(|total| (downloaded_bytes as f32 / total as f32 * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);

    app.emit(
        "speaker-model-download-progress",
        DownloadProgress {
            model_id: model_id.to_string(),
            downloaded_bytes,
            total_bytes,
            progress,
        },
    )?;
    Ok(())
}
