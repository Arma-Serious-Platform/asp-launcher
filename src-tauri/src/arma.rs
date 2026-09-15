use std::path::{Path, PathBuf};

use crate::settings::Settings;

pub fn is_arma_dir(path: &Path) -> bool {
    arma_exe(path).is_some()
}

pub fn arma_exe(path: &Path) -> Option<PathBuf> {
    let x64 = path.join("arma3_x64.exe");
    if x64.is_file() {
        return Some(x64);
    }
    let x86 = path.join("arma3.exe");
    if x86.is_file() {
        return Some(x86);
    }
    None
}

pub fn detect_arma3_path() -> Option<String> {
    #[cfg(windows)]
    {
        detect_windows().map(|p| p.to_string_lossy().to_string())
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn validate_arma_path(path: &str) -> Result<String, String> {
    let path = PathBuf::from(path.trim());
    if !path.is_dir() {
        return Err("Вказаний шлях не є текою".into());
    }
    if !is_arma_dir(&path) {
        return Err("У теці немає arma3_x64.exe або arma3.exe".into());
    }
    Ok(path.to_string_lossy().to_string())
}

pub fn launch_game(settings: &Settings) -> Result<(), String> {
    #[cfg(windows)]
    {
        launch_windows(settings)
    }
    #[cfg(not(windows))]
    {
        let _ = settings;
        Err("Запуск Arma 3 доступний лише на Windows".into())
    }
}

#[cfg(windows)]
fn launch_windows(settings: &Settings) -> Result<(), String> {
    use std::process::Command;

    let arma_path = PathBuf::from(settings.arma3_path.trim());
    let exe = arma_exe(&arma_path).ok_or_else(|| {
        "Не знайдено arma3_x64.exe. Перевірте шлях до гри в налаштуваннях.".to_string()
    })?;

    let mods_path = PathBuf::from(settings.resolved_mods_path());
    let same_dir = paths_equal(&arma_path, &mods_path);

    let mut cmd = Command::new(&exe);
    cmd.current_dir(&arma_path);
    cmd.arg("-noSplash");

    if !settings.selected_mods.is_empty() {
        let joined = settings
            .selected_mods
            .iter()
            .filter(|name| !name.trim().is_empty())
            .map(|name| {
                if same_dir {
                    name.clone()
                } else {
                    mods_path.join(name).to_string_lossy().to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(";");
        if !joined.is_empty() {
            cmd.arg(format!("-mod={joined}"));
        }
    }

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Не вдалося запустити Arma 3: {e}"))
}

#[cfg(windows)]
fn paths_equal(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    canon(a) == canon(b)
}

#[cfg(windows)]
fn detect_windows() -> Option<PathBuf> {
    if let Some(path) = from_registry() {
        if is_arma_dir(&path) {
            return Some(path);
        }
    }
    for library in steam_libraries() {
        let candidate = library.join("steamapps").join("common").join("Arma 3");
        if is_arma_dir(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(windows)]
fn from_registry() -> Option<PathBuf> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    const KEYS: &[&str] = &[
        r"SOFTWARE\WOW6432Node\Bohemia Interactive\Arma 3",
        r"SOFTWARE\Bohemia Interactive\Arma 3",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 107410",
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 107410",
    ];
    const VALUES: &[&str] = &["main", "InstallLocation"];

    for key_path in KEYS {
        if let Ok(key) = hklm.open_subkey(key_path) {
            for value_name in VALUES {
                if let Ok(value) = key.get_value::<String, _>(value_name) {
                    let path = PathBuf::from(value);
                    if is_arma_dir(&path) {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

#[cfg(windows)]
fn steam_libraries() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from(r"C:\Program Files (x86)\Steam"),
        PathBuf::from(r"C:\Program Files\Steam"),
    ];
    let mut libraries = roots.clone();

    for root in roots.split_off(0) {
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = std::fs::read_to_string(vdf) {
            for path in parse_libraryfolders(&text) {
                if !libraries.iter().any(|existing| existing == &path) {
                    libraries.push(path);
                }
            }
        }
    }
    libraries
}

#[cfg(windows)]
fn parse_libraryfolders(text: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if !line.starts_with("\"path\"") {
            continue;
        }
        if let Some(value) = line.splitn(3, '"').nth(3).or_else(|| {
            line.rsplit('"').nth(1)
        }) {
            let cleaned = value.replace("\\\\", "\\");
            if !cleaned.is_empty() && cleaned != "path" {
                paths.push(PathBuf::from(cleaned));
            }
        }
    }
    paths
}
