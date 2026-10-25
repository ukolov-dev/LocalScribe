//! Private JSON-lines transport for the native macOS interface; no WebView or tray.
use crate::{
    errors::{invalid, AppResult},
    AppState,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Listener, Manager};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: String,
    command: String,
    #[serde(default)]
    args: Value,
}

fn output(value: Value) {
    static STDOUT: OnceLock<Mutex<io::Stdout>> = OnceLock::new();
    if let Ok(mut stream) = STDOUT.get_or_init(|| Mutex::new(io::stdout())).lock() {
        let _ = writeln!(stream, "{value}");
        let _ = stream.flush();
    }
}
fn argument<T: serde::de::DeserializeOwned>(args: &Value, key: &str) -> AppResult<T> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|_| invalid(format!("Missing or invalid argument: {key}")))
}
async fn dispatch(app: AppHandle, request: &Request) -> AppResult<Value> {
    let a = &request.args;
    let result = match request.command.as_str() {
        "health" => json!({"status":"ok"}),
        "list_models" => serde_json::to_value(crate::list_models(app)?)?,
        "download_model" => {
            serde_json::to_value(crate::download_model(app, argument(a, "modelId")?).await?)?
        }
        "delete_model" => serde_json::to_value(crate::delete_model(app, argument(a, "modelId")?)?)?,
        "select_custom_model" => {
            serde_json::to_value(crate::select_custom_model(app, argument(a, "path")?)?)?
        }
        "list_speaker_models" => serde_json::to_value(crate::list_speaker_models(app)?)?,
        "download_speaker_model" => serde_json::to_value(
            crate::download_speaker_model(app, argument(a, "modelId")?).await?,
        )?,
        "delete_speaker_model" => {
            serde_json::to_value(crate::delete_speaker_model(app, argument(a, "modelId")?)?)?
        }
        "select_custom_speaker_model" => serde_json::to_value(crate::select_custom_speaker_model(
            app,
            argument(a, "path")?,
        )?)?,
        "transcribe_file" => {
            let mut options: crate::types::TranscribeOptions = argument(a, "options")?;
            options.speaker_mode = Some("auto".into());
            serde_json::to_value(crate::transcribe_file(app.clone(), app.state(), options).await?)?
        }
        "cancel_transcription" => serde_json::to_value(crate::cancel_transcription(
            app.state(),
            argument(a, "jobId")?,
        )?)?,
        "export_transcript" => {
            serde_json::to_value(crate::export_transcript(argument(a, "request")?)?)?
        }
        "claim_integration_request" => {
            serde_json::to_value(crate::claim_integration_request(app)?)?
        }
        "complete_integration_request" => {
            serde_json::to_value(crate::complete_integration_request(
                app,
                argument(a, "jobId")?,
                argument(a, "result")?,
            )?)?
        }
        "fail_integration_request" => {
            crate::fail_integration_request(app, argument(a, "jobId")?, argument(a, "message")?)?;
            Value::Null
        }
        _ => return Err(invalid("Unknown native command.")),
    };
    Ok(result)
}

pub fn run(mut context: tauri::Context<tauri::Wry>) {
    context.config_mut().app.windows.clear();
    tauri::Builder::default().manage(AppState::default()).setup(|app| {
        #[cfg(target_os="macos")]
        app.set_activation_policy(tauri::ActivationPolicy::Prohibited);
        crate::models::models_dir(app.handle())?;
        crate::integration::initialize(app.handle())?;
        crate::meeting_api::start(app.handle())?;
        for name in ["transcription-progress","transcription-segment","model-download-progress","speaker-model-download-progress"] {
            app.listen_any(name, move |event| {
                if let Ok(payload) = serde_json::from_str::<Value>(event.payload()) {
                    output(json!({"event":name,"payload":payload}));
                }
            });
        }
        let app = app.handle().clone();
        std::thread::spawn(move || {
            // A request must fit in memory; paths/options are small, transcripts are responses.
            let mut input = io::stdin().lock();
            loop {
                let mut line = Vec::new();
                let read = input.by_ref().take(1024 * 1024 + 1).read_until(b'\n', &mut line);
                match read {
                    Ok(0) | Err(_) => break,
                    Ok(_) if line.len() > 1024 * 1024 || line.last() != Some(&b'\n') => break,
                    _ => {}
                }
                match serde_json::from_slice::<Request>(&line) {
                    Ok(request) => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let response = match dispatch(app, &request).await {
                                Ok(result) => json!({"id":request.id,"result":result}),
                                Err(error) => json!({"id":request.id,"error":error}),
                            };
                            output(response);
                        });
                    }
                    Err(_) => output(json!({"error":{"code":"INVALID_REQUEST","message":"Invalid native request."}})),
                }
            }
            app.exit(0);
        });
        Ok(())
    }).run(context).expect("Cannot start LocalScribe engine");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_arguments_and_unknown_envelope_fields() {
        assert!(argument::<String>(&json!({"path":12}), "path").is_err());
        assert!(serde_json::from_value::<Request>(
            json!({"id":"1","command":"health","extra":true})
        )
        .is_err());
    }
}
