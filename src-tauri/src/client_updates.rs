use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, State, ipc::Channel};
use tauri_plugin_updater::{Update, UpdaterExt};
use url::Url;

use crate::InstanceMutation;
use crate::client_update_policy::{validate_download_url, verify_artifact};

const ENDPOINT: &str =
    "https://github.com/Mars-Command/Client-Launcher/releases/latest/download/latest.json";
const PUBLIC_KEY: &str = match option_env!("MARS_UPDATER_PUBLIC_KEY") {
    Some(value) => value,
    None => "",
};
const MAX_UPDATE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Default)]
pub struct ClientUpdateState(tokio::sync::Mutex<Option<Update>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    notes: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum UpdateProgress {
    Downloading { downloaded: u64, total: Option<u64> },
    Installing,
}

fn require_supported_installation() -> Result<(), String> {
    if PUBLIC_KEY.is_empty() {
        return Err(
            "Client updates are disabled: this development build has no signing public key".into(),
        );
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("APPIMAGE").is_none() {
        return Err("In-app updates on Linux require the AppImage distribution. Update Debian packages through your package manager or install the official AppImage.".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn check_client_update(
    app: AppHandle,
    state: State<'_, ClientUpdateState>,
) -> Result<Option<UpdateInfo>, String> {
    let mut pending = state
        .0
        .try_lock()
        .map_err(|_| "A client update operation is already running")?;
    *pending = None;
    require_supported_installation()?;
    let endpoint =
        Url::parse(ENDPOINT).map_err(|err| format!("Invalid built-in update endpoint: {err}"))?;
    let update = app
        .updater_builder()
        .pubkey(PUBLIC_KEY)
        .endpoints(vec![endpoint])
        .map_err(|err| format!("Could not configure client updates: {err}"))?
        .timeout(Duration::from_secs(20))
        .version_comparator(|current, release| {
            release.version > current && release.version.pre.is_empty()
        })
        .configure_client(|client| client.https_only(true))
        .build()
        .map_err(|err| format!("Could not initialize client updates: {err}"))?
        .check()
        .await
        .map_err(|err| format!("Could not check for client updates: {err}"))?;
    let Some(update) = update else {
        return Ok(None);
    };
    validate_download_url(&update.download_url, &update.version)?;
    let info = UpdateInfo {
        version: update.version.clone(),
        notes: update.body.clone(),
    };
    *pending = Some(update);
    Ok(Some(info))
}

#[tauri::command]
pub async fn install_client_update(
    app: AppHandle,
    state: State<'_, ClientUpdateState>,
    mutation: State<'_, InstanceMutation>,
    expected_version: String,
    confirmed: bool,
    progress: Channel<UpdateProgress>,
) -> Result<(), String> {
    if !confirmed {
        return Err("Confirm the client update before downloading or installing".into());
    }
    let _mutation = mutation
        .0
        .try_lock()
        .map_err(|_| "Wait for the current game installation operation to finish")?;
    let mut pending = state
        .0
        .try_lock()
        .map_err(|_| "A client update operation is already running")?;
    require_supported_installation()?;
    let update = pending
        .as_ref()
        .ok_or("Check for client updates before installing")?;
    if update.version != expected_version {
        return Err("The available client version changed; check again before installing".into());
    }
    validate_download_url(&update.download_url, &update.version)?;
    let update = pending
        .take()
        .ok_or("The pending client update is unavailable")?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|err| format!("Could not create the update download client: {err}"))?;
    let response = client
        .get(update.download_url.clone())
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|err| format!("Client update download failed: {err}"))?;
    let total = response.content_length();
    if total.is_some_and(|length| length > MAX_UPDATE_BYTES) {
        return Err("Client update exceeds the 256 MiB download limit".into());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|err| format!("Client update download was interrupted: {err}"))?;
        if bytes.len() as u64 + chunk.len() as u64 > MAX_UPDATE_BYTES {
            return Err("Client update exceeds the 256 MiB download limit".into());
        }
        bytes.extend_from_slice(&chunk);
        progress
            .send(UpdateProgress::Downloading {
                downloaded: bytes.len() as u64,
                total,
            })
            .map_err(|err| {
                format!("Could not report update progress; installation cancelled: {err}")
            })?;
    }
    // Verification binds both the artifact and the announced version before any installer runs.
    verify_artifact(&bytes, PUBLIC_KEY, &update.signature, &update.version).map_err(|err| {
        format!("Client update verification failed; nothing was installed: {err}")
    })?;
    progress.send(UpdateProgress::Installing).map_err(|err| {
        format!("Could not report installation status; installation cancelled: {err}")
    })?;
    tauri::async_runtime::spawn_blocking(move || update.install(bytes))
        .await
        .map_err(|err| format!("Client update installer task failed: {err}"))?
        .map_err(|err| format!("Client update installation failed: {err}"))?;
    app.restart();
}
