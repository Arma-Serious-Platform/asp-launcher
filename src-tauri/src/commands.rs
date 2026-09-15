use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde_json::Value;
use tauri::{AppHandle, State};

use crate::arma;
use crate::ftp;
use crate::settings::{self, Settings};

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub cancel: AtomicBool,
    pub syncing: AtomicBool,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: Mutex::new(settings),
            cancel: AtomicBool::new(false),
            syncing: AtomicBool::new(false),
        }
    }

    pub fn snapshot(&self) -> Result<Settings, String> {
        self.settings
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| "Не вдалося прочитати налаштування".to_string())
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    state.snapshot()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, String> {
    let settings = settings::normalize_settings(settings);
    settings.save(&app)?;
    let mut guard = state
        .settings
        .lock()
        .map_err(|_| "Не вдалося зберегти налаштування".to_string())?;
    *guard = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn detect_arma_path() -> Option<String> {
    arma::detect_arma3_path()
}

#[tauri::command]
pub fn validate_arma_path(path: String) -> Result<String, String> {
    arma::validate_arma_path(&path)
}

#[tauri::command]
pub fn launch_game(state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.snapshot()?;
    arma::launch_game(&settings)
}

#[tauri::command]
pub async fn ftp_test_connection(state: State<'_, AppState>) -> Result<(), String> {
    let settings = state.snapshot()?;
    ftp::test_connection(&settings.ftp).await
}

#[tauri::command]
pub async fn ftp_list_mods(state: State<'_, AppState>) -> Result<Vec<ftp::RemoteMod>, String> {
    let settings = state.snapshot()?;
    ftp::list_mods(&settings.ftp).await
}

#[tauri::command]
pub async fn sync_status(state: State<'_, AppState>) -> Result<ftp::SyncStatus, String> {
    let settings = state.snapshot()?;
    ftp::sync_status(&settings).await
}

#[tauri::command]
pub async fn start_sync(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if state
        .syncing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Завантаження вже виконується".into());
    }
    let settings = match state.snapshot() {
        Ok(settings) => settings,
        Err(err) => {
            state.syncing.store(false, Ordering::SeqCst);
            return Err(err);
        }
    };

    let result = ftp::start_sync(&app, &settings, &state.cancel).await;
    state.syncing.store(false, Ordering::SeqCst);
    state.cancel.store(false, Ordering::SeqCst);
    result
}

#[tauri::command]
pub fn cancel_sync(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub async fn fetch_weekends() -> Result<Value, String> {
    let response = reqwest::get(
        "https://service.beta.vtg.in.ua/api/weekends?skip=0&published=true&take=1",
    )
    .await
    .map_err(|e| format!("Не вдалося отримати анонси: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("API анонсів повернуло {}", response.status()));
    }
    response
        .json::<Value>()
        .await
        .map_err(|e| format!("Не вдалося розібрати анонси: {e}"))
}
