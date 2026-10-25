use crate::errors::{AppError, AppResult};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn data_dir(app: &AppHandle) -> AppResult<PathBuf> {
    // Keep existing models, recordings and settings when the macOS bundle identity changes.
    #[cfg(target_os = "macos")]
    let path = app
        .path()
        .data_dir()
        .map(|root| root.join("com.localscribe.desktop"));
    #[cfg(not(target_os = "macos"))]
    let path = app.path().app_data_dir();
    path.map_err(|error| AppError::Config(error.to_string()))
}
