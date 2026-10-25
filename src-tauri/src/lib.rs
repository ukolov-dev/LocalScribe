mod audio;
mod diarization;
mod downloader;
mod errors;
pub mod export;
mod integration;
mod meeting_api;
mod models;
mod native_worker;
mod settings;
mod speaker_models;
mod storage;
mod transcription;
mod tray;
pub mod types;

use crate::{
    errors::{AppError, AppResult},
    types::{
        ExportRequest, IntegrationCompletion, IntegrationRequest, ModelInfo, SpeakerModelInfo,
        TranscribeOptions, TranscriptionError, TranscriptionResult,
    },
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[derive(Default)]
pub struct AppState {
    active_jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

#[tauri::command]
fn list_models(app: AppHandle) -> AppResult<Vec<ModelInfo>> {
    models::list_models(&app)
}

#[tauri::command]
async fn download_model(app: AppHandle, model_id: String) -> AppResult<ModelInfo> {
    downloader::download_model(app, model_id).await
}

#[tauri::command]
fn delete_model(app: AppHandle, model_id: String) -> AppResult<Vec<ModelInfo>> {
    models::delete_model(&app, &model_id)
}

#[tauri::command]
fn select_custom_model(app: AppHandle, path: String) -> AppResult<ModelInfo> {
    models::select_custom_model(&app, types::SelectCustomModelRequest { path })
}

#[tauri::command]
fn list_speaker_models(app: AppHandle) -> AppResult<Vec<SpeakerModelInfo>> {
    speaker_models::list_speaker_models(&app)
}

#[tauri::command]
async fn download_speaker_model(app: AppHandle, model_id: String) -> AppResult<SpeakerModelInfo> {
    downloader::download_speaker_model(app, model_id).await
}

#[tauri::command]
fn delete_speaker_model(app: AppHandle, model_id: String) -> AppResult<Vec<SpeakerModelInfo>> {
    speaker_models::delete_speaker_model(&app, &model_id)
}

#[tauri::command]
fn select_custom_speaker_model(app: AppHandle, path: String) -> AppResult<SpeakerModelInfo> {
    speaker_models::select_custom_speaker_model(&app, types::SelectCustomModelRequest { path })
}

#[tauri::command]
async fn transcribe_file(
    app: AppHandle,
    state: State<'_, AppState>,
    mut options: TranscribeOptions,
) -> AppResult<TranscriptionResult> {
    let job_id = options
        .job_id
        .clone()
        .filter(|job_id| !job_id.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    options.job_id = Some(job_id.clone());

    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut jobs = state
            .active_jobs
            .lock()
            .map_err(|_| AppError::Unexpected("Job state lock is poisoned.".to_string()))?;
        jobs.insert(job_id.clone(), Arc::clone(&cancel_flag));
    }

    let app_for_task = app.clone();
    let job_id_for_task = job_id.clone();
    let result = async {
        let download = downloader::download_speaker_model(
            app.clone(),
            speaker_models::NEURAL_SPEAKER_MODEL_ID.into(),
        );
        app.emit(
            "transcription-progress",
            types::TranscriptionProgress {
                job_id: job_id.clone(),
                phase: "downloading-speaker-models".into(),
                progress: 0.0,
            },
        )?;
        tokio::select! {
            model = download => { model?; },
            _ = async {
                while !cancel_flag.load(Ordering::Relaxed) {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            } => return Err(AppError::Cancelled),
        }
        tauri::async_runtime::spawn_blocking(move || {
            transcription::transcribe(app_for_task, job_id_for_task, options, cancel_flag)
        })
        .await
        .map_err(|error| AppError::Unexpected(error.to_string()))
        .and_then(|result| result)
    }
    .await;

    {
        let mut jobs = state
            .active_jobs
            .lock()
            .map_err(|_| AppError::Unexpected("Job state lock is poisoned.".to_string()))?;
        jobs.remove(&job_id);
    }

    if let Err(error) = &result {
        emit_transcription_error(&app, &job_id, error);
    }

    result
}

#[tauri::command]
fn cancel_transcription(state: State<'_, AppState>, job_id: String) -> AppResult<bool> {
    let jobs = state
        .active_jobs
        .lock()
        .map_err(|_| AppError::Unexpected("Job state lock is poisoned.".to_string()))?;

    if let Some(cancel_flag) = jobs.get(&job_id) {
        cancel_flag.store(true, Ordering::Relaxed);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
fn export_transcript(request: ExportRequest) -> AppResult<String> {
    export::export_transcript(request)
}

#[tauri::command]
fn claim_integration_request(app: AppHandle) -> AppResult<Option<IntegrationRequest>> {
    integration::claim_next(&app)
}

#[tauri::command]
fn complete_integration_request(
    app: AppHandle,
    job_id: String,
    result: TranscriptionResult,
) -> AppResult<IntegrationCompletion> {
    integration::complete(&app, &job_id, &result)
}

#[tauri::command]
fn fail_integration_request(app: AppHandle, job_id: String, message: String) -> AppResult<()> {
    integration::fail(&app, &job_id, &message)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if std::env::args().nth(1).as_deref() == Some("--diarize") {
        if let Err(error) = diarization::run_cli(&std::env::args().skip(2).collect::<Vec<_>>()) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    let context = tauri::generate_context!();
    if std::env::args().nth(1).as_deref() == Some("--native-worker") {
        native_worker::run(context);
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            models::models_dir(app.handle())?;
            speaker_models::speaker_models_dir(app.handle())?;
            integration::initialize(app.handle())?;
            meeting_api::start(app.handle())?;
            tray::setup(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = window.hide() {
                    eprintln!("Cannot hide LocalScribe: {error}");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_models,
            download_model,
            delete_model,
            select_custom_model,
            list_speaker_models,
            download_speaker_model,
            delete_speaker_model,
            select_custom_speaker_model,
            transcribe_file,
            cancel_transcription,
            export_transcript,
            claim_integration_request,
            complete_integration_request,
            fail_integration_request
        ])
        .run(context)
        .expect("error while running LocalScribe");
}

fn emit_transcription_error(app: &AppHandle, job_id: &str, error: &AppError) {
    let _ = app.emit(
        "transcription-error",
        TranscriptionError {
            job_id: job_id.to_string(),
            code: error.code().to_string(),
            message: error.to_string(),
        },
    );
}
