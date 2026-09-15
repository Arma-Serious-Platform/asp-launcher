use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const DEFAULT_FTP_URL: &str = "ftp://88.99.69.243/Repo/WOG/.a3s/autoconfig";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FtpSettings {
    pub source_url: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub remote_path: String,
    pub passive: bool,
}

impl Default for FtpSettings {
    fn default() -> Self {
        Self {
            source_url: DEFAULT_FTP_URL.into(),
            host: String::new(),
            port: 21,
            username: String::new(),
            password: String::new(),
            remote_path: "/".into(),
            passive: true,
        }
    }
}

impl FtpSettings {
    pub fn has_source(&self) -> bool {
        !self.source_url.trim().is_empty() || !self.host.trim().is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub arma3_path: String,
    pub mods_path: String,
    pub ftp: FtpSettings,
    pub selected_mods: Vec<String>,
}

impl Settings {
    pub fn file_path(app: &AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|e| format!("Не вдалося визначити теку налаштувань: {e}"))?;
        Ok(dir.join("settings.json"))
    }

    pub fn load(app: &AppHandle) -> Self {
        let Ok(path) = Self::file_path(app) else {
            return Self::default();
        };
        let file_missing = !path.exists();
        let mut settings = if let Ok(raw) = fs::read_to_string(&path) {
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            Self::default()
        };
        let filled = apply_default_source(&mut settings);
        if file_missing || filled {
            let _ = settings.save(app);
        }
        settings
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let path = Self::file_path(app)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Не вдалося створити теку налаштувань: {e}"))?;
        }
        let raw = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Не вдалося серіалізувати налаштування: {e}"))?;
        fs::write(&path, raw).map_err(|e| format!("Не вдалося зберегти налаштування: {e}"))
    }

    pub fn resolved_mods_path(&self) -> String {
        if self.mods_path.trim().is_empty() {
            self.arma3_path.clone()
        } else {
            self.mods_path.clone()
        }
    }
}

pub fn apply_default_source(settings: &mut Settings) -> bool {
    if settings.ftp.host.trim().is_empty() && settings.ftp.source_url.trim().is_empty() {
        settings.ftp.source_url = DEFAULT_FTP_URL.into();
        true
    } else {
        false
    }
}

pub fn normalize_settings(mut settings: Settings) -> Settings {
    apply_default_source(&mut settings);
    if settings.ftp.port == 0 {
        settings.ftp.port = 21;
    }
    if settings.ftp.remote_path.trim().is_empty() {
        settings.ftp.remote_path = "/".into();
    }
    settings
}
