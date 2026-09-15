use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use serde::Serialize;
use suppaftp::list::File;
use suppaftp::tokio::AsyncFtpStream;
use suppaftp::types::FileType;
use suppaftp::Mode;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::settings::{FtpSettings, Settings};

const PROGRESS_EVENT: &str = "download-progress";
const CHUNK: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteMod {
    pub name: String,
    pub remote_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub ready: bool,
    pub bytes_missing: u64,
    pub files_missing: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub phase: String,
    pub mod_name: Option<String>,
    pub current_file: Option<String>,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub files_done: u32,
    pub files_total: u32,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteFile {
    rel_path: String,
    size: u64,
    mtime: Option<DateTime<Utc>>,
}

pub async fn test_connection(ftp: &FtpSettings) -> Result<(), String> {
    let mut stream = connect(ftp).await?;
    let _ = stream.quit().await;
    Ok(())
}

pub async fn list_mods(ftp: &FtpSettings) -> Result<Vec<RemoteMod>, String> {
    let mut stream = connect(ftp).await?;
    let entries = list_entries(&mut stream, None).await?;
    let mut mods = Vec::new();

    for entry in entries {
        if !entry.is_directory() || is_dot(&entry) {
            continue;
        }
        let name = file_name(&entry);
        if !is_safe_name(&name) {
            continue;
        }
        let remote_bytes = folder_size(&mut stream, &name).await.unwrap_or(0);
        mods.push(RemoteMod { name, remote_bytes });
    }

    mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let _ = stream.quit().await;
    Ok(mods)
}

pub async fn sync_status(settings: &Settings) -> Result<SyncStatus, String> {
    if settings.selected_mods.is_empty() {
        return Ok(SyncStatus {
            ready: true,
            bytes_missing: 0,
            files_missing: 0,
        });
    }
    if settings.ftp.host.trim().is_empty() {
        return Err("Спочатку вкажіть FTP-сервер у налаштуваннях".into());
    }

    let mut stream = connect(&settings.ftp).await?;
    let mods_root = PathBuf::from(settings.resolved_mods_path());
    let mut bytes_missing = 0_u64;
    let mut files_missing = 0_u32;

    for mod_name in &settings.selected_mods {
        ensure_safe_name(mod_name)?;
        let remote = walk_remote(&mut stream, mod_name).await?;
        let needed = files_to_download(&mods_root.join(mod_name), &remote);
        files_missing += needed.len() as u32;
        bytes_missing += needed.iter().map(|f| f.size).sum::<u64>();
    }

    let _ = stream.quit().await;
    Ok(SyncStatus {
        ready: files_missing == 0,
        bytes_missing,
        files_missing,
    })
}

pub async fn start_sync(
    app: &AppHandle,
    settings: &Settings,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if settings.ftp.host.trim().is_empty() {
        return Err("Спочатку вкажіть FTP-сервер у налаштуваннях".into());
    }
    if settings.selected_mods.is_empty() {
        emit_progress(
            app,
            &DownloadProgress {
                phase: "done".into(),
                mod_name: None,
                current_file: None,
                bytes_done: 0,
                bytes_total: 0,
                files_done: 0,
                files_total: 0,
                message: Some("Немає вибраних модів".into()),
            },
        );
        return Ok(());
    }

    cancel.store(false, Ordering::SeqCst);
    emit_progress(
        app,
        &DownloadProgress {
            phase: "listing".into(),
            mod_name: None,
            current_file: None,
            bytes_done: 0,
            bytes_total: 0,
            files_done: 0,
            files_total: 0,
            message: Some("Сканування FTP...".into()),
        },
    );

    let mut stream = connect(&settings.ftp).await?;
    let mods_root = PathBuf::from(settings.resolved_mods_path());
    std::fs::create_dir_all(&mods_root)
        .map_err(|e| format!("Не вдалося створити теку модів: {e}"))?;

    let mut planned: Vec<(String, RemoteFile)> = Vec::new();
    for mod_name in &settings.selected_mods {
        check_cancel(cancel)?;
        ensure_safe_name(mod_name)?;
        let remote = walk_remote(&mut stream, mod_name).await?;
        for file in files_to_download(&mods_root.join(mod_name), &remote) {
            planned.push((mod_name.clone(), file));
        }
    }

    let bytes_total: u64 = planned.iter().map(|(_, f)| f.size).sum();
    let files_total = planned.len() as u32;
    let mut bytes_done = 0_u64;
    let mut files_done = 0_u32;

    if planned.is_empty() {
        emit_progress(
            app,
            &DownloadProgress {
                phase: "done".into(),
                mod_name: None,
                current_file: None,
                bytes_done: 0,
                bytes_total: 0,
                files_done: 0,
                files_total: 0,
                message: Some("Усі дані вже завантажені".into()),
            },
        );
        let _ = stream.quit().await;
        return Ok(());
    }

    for (mod_name, file) in planned {
        check_cancel(cancel)?;
        emit_progress(
            app,
            &DownloadProgress {
                phase: "downloading".into(),
                mod_name: Some(mod_name.clone()),
                current_file: Some(file.rel_path.clone()),
                bytes_done,
                bytes_total,
                files_done,
                files_total,
                message: None,
            },
        );

        let remote_path = format!("{mod_name}/{}", file.rel_path);
        let local_path = mods_root.join(&mod_name).join(&file.rel_path);
        download_file(&mut stream, &remote_path, &local_path, cancel, |chunk| {
            bytes_done += chunk;
            emit_progress(
                app,
                &DownloadProgress {
                    phase: "downloading".into(),
                    mod_name: Some(mod_name.clone()),
                    current_file: Some(file.rel_path.clone()),
                    bytes_done,
                    bytes_total,
                    files_done,
                    files_total,
                    message: None,
                },
            );
        })
        .await?;

        files_done += 1;
    }

    emit_progress(
        app,
        &DownloadProgress {
            phase: "done".into(),
            mod_name: None,
            current_file: None,
            bytes_done,
            bytes_total,
            files_done,
            files_total,
            message: Some("Завантаження завершено".into()),
        },
    );
    let _ = stream.quit().await;
    Ok(())
}

async fn connect(ftp: &FtpSettings) -> Result<AsyncFtpStream, String> {
    if ftp.host.trim().is_empty() {
        return Err("Вкажіть адресу FTP".into());
    }
    let addr = format!("{}:{}", ftp.host.trim(), ftp.port);
    let mut stream = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| format!("Не вдалося підключитися до FTP ({addr}): {e}"))?;

    stream.set_mode(if ftp.passive {
        Mode::Passive
    } else {
        Mode::Active
    });

    let user = if ftp.username.trim().is_empty() {
        "anonymous"
    } else {
        ftp.username.trim()
    };
    stream
        .login(user, &ftp.password)
        .await
        .map_err(|e| format!("Помилка авторизації FTP: {e}"))?;
    stream
        .transfer_type(FileType::Binary)
        .await
        .map_err(|e| format!("Не вдалося увімкнути бінарний режим: {e}"))?;

    let remote = ftp.remote_path.trim();
    if !remote.is_empty() && remote != "/" {
        stream
            .cwd(remote)
            .await
            .map_err(|e| format!("Не вдалося відкрити теку {remote}: {e}"))?;
    }
    Ok(stream)
}

async fn list_entries(
    stream: &mut AsyncFtpStream,
    path: Option<&str>,
) -> Result<Vec<File>, String> {
    let lines = stream
        .list(path)
        .await
        .map_err(|e| format!("Не вдалося отримати список файлів: {e}"))?;
    Ok(lines.iter().filter_map(|line| line.parse::<File>().ok()).collect())
}

async fn folder_size(stream: &mut AsyncFtpStream, folder: &str) -> Result<u64, String> {
    let files = walk_remote(stream, folder).await?;
    Ok(files.iter().map(|f| f.size).sum())
}

async fn walk_remote(stream: &mut AsyncFtpStream, folder: &str) -> Result<Vec<RemoteFile>, String> {
    let mut files = Vec::new();
    walk_dir(stream, folder, "", &mut files).await?;
    Ok(files)
}

async fn walk_dir(
    stream: &mut AsyncFtpStream,
    root: &str,
    rel: &str,
    out: &mut Vec<RemoteFile>,
) -> Result<(), String> {
    let list_path = if rel.is_empty() {
        root.to_string()
    } else {
        format!("{root}/{rel}")
    };
    let entries = list_entries(stream, Some(&list_path)).await?;
    for entry in entries {
        if is_dot(&entry) {
            continue;
        }
        let name = file_name(&entry);
        if !is_safe_name(&name) {
            continue;
        }
        let child_rel = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if entry.is_directory() {
            Box::pin(walk_dir(stream, root, &child_rel, out)).await?;
        } else {
            out.push(RemoteFile {
                rel_path: child_rel,
                size: entry.size() as u64,
                mtime: Some(DateTime::<Utc>::from(entry.modified())),
            });
        }
    }
    Ok(())
}

fn files_to_download(local_root: &Path, remote: &[RemoteFile]) -> Vec<RemoteFile> {
    remote
        .iter()
        .filter(|file| needs_download(local_root, file))
        .cloned()
        .collect()
}

fn needs_download(local_root: &Path, file: &RemoteFile) -> bool {
    let local = local_root.join(&file.rel_path);
    let Ok(meta) = std::fs::metadata(&local) else {
        return true;
    };
    if !meta.is_file() {
        return true;
    }
    if meta.len() != file.size {
        return true;
    }
    if let (Some(remote_mtime), Ok(modified)) = (file.mtime, meta.modified()) {
        let local_mtime = DateTime::<Utc>::from(modified);
        if remote_mtime.timestamp() > local_mtime.timestamp() + 2 {
            return true;
        }
    }
    false
}

async fn download_file<F>(
    stream: &mut AsyncFtpStream,
    remote_path: &str,
    local_path: &Path,
    cancel: &AtomicBool,
    mut on_chunk: F,
) -> Result<(), String>
where
    F: FnMut(u64),
{
    if let Some(parent) = local_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Не вдалося створити теку {}: {e}", parent.display()))?;
    }

    let tmp = local_path.with_extension(format!(
        "{}part",
        local_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{e}."))
            .unwrap_or_default()
    ));

    let mut reader = stream
        .retr_as_stream(remote_path)
        .await
        .map_err(|e| format!("Не вдалося завантажити {remote_path}: {e}"))?;
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("Не вдалося створити файл {}: {e}", tmp.display()))?;
    let mut buf = vec![0_u8; CHUNK];
    let mut download_err: Option<String> = None;

    loop {
        if let Err(err) = check_cancel(cancel) {
            download_err = Some(err);
            break;
        }
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                if let Err(e) = file.write_all(&buf[..n]).await {
                    download_err = Some(format!("Помилка запису {}: {e}", tmp.display()));
                    break;
                }
                on_chunk(n as u64);
            }
            Err(e) => {
                download_err = Some(format!("Помилка читання {remote_path}: {e}"));
                break;
            }
        }
    }

    if download_err.is_none() {
        if let Err(e) = file.flush().await {
            download_err = Some(format!("Не вдалося зберегти {}: {e}", tmp.display()));
        }
    }
    drop(file);

    if let Err(err) = reader.finish().await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(format!("Не вдалося завершити завантаження {remote_path}: {err}"));
    }

    if let Some(err) = download_err {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(err);
    }

    tokio::fs::rename(&tmp, local_path)
        .await
        .map_err(|e| format!("Не вдалося перемістити файл {}: {e}", local_path.display()))
}

fn file_name(file: &File) -> String {
    file.name().to_string()
}

fn is_dot(file: &File) -> bool {
    matches!(file.name(), "." | "..")
}

fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
        && !Path::new(name).is_absolute()
}

fn ensure_safe_name(name: &str) -> Result<(), String> {
    if is_safe_name(name) {
        Ok(())
    } else {
        Err(format!("Некоректна назва мода: {name}"))
    }
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        Err("Завантаження скасовано".into())
    } else {
        Ok(())
    }
}

fn emit_progress(app: &AppHandle, payload: &DownloadProgress) {
    let _ = app.emit(PROGRESS_EVENT, payload);
}
