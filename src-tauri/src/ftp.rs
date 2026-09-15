use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sha1::{Digest, Sha1};
use suppaftp::list::File;
use suppaftp::tokio::AsyncFtpStream;
use suppaftp::types::FileType;
use suppaftp::Mode;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::a3s::{self, SourceUrl, SyncInventory};
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FtpProbe {
    pub mode: String,
    pub host: String,
    pub port: u16,
    pub remote_path: String,
    pub has_sync: bool,
    pub repository_name: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteFile {
    rel_path: String,
    size: u64,
    mtime: Option<DateTime<Utc>>,
    sha1: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedRepo {
    source: SourceUrl,
    username: String,
    password: String,
    passive: bool,
    mode: &'static str,
    inventory: Option<SyncInventory>,
    repository_name: Option<String>,
}

pub async fn test_connection(ftp: &FtpSettings) -> Result<FtpProbe, String> {
    let repo = resolve_repo(ftp).await?;
    if repo.source.scheme == "ftp" && repo.inventory.is_none() {
        let mut stream = connect(&repo).await?;
        let _ = stream.quit().await;
    }
    Ok(probe_from(&repo))
}

pub async fn list_mods(ftp: &FtpSettings) -> Result<Vec<RemoteMod>, String> {
    let repo = resolve_repo(ftp).await?;
    if let Some(inventory) = &repo.inventory {
        let mut mods: Vec<RemoteMod> = inventory
            .addons
            .iter()
            .map(|addon| RemoteMod {
                name: addon.name.clone(),
                remote_bytes: inventory.remote_bytes(&addon.name),
            })
            .collect();
        mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        return Ok(mods);
    }
    if repo.source.scheme != "ftp" {
        return Err("Немає A3S sync, а HTTP не підтримує LIST".into());
    }

    let mut stream = connect(&repo).await?;
    let entries = list_entries(&mut stream, None).await?;
    let mut mods = Vec::new();
    for entry in entries {
        if !entry.is_directory() || is_dot(&entry) {
            continue;
        }
        let name = file_name(&entry);
        if !is_safe_name(&name) || is_metadata_dir(&name) {
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
    if !settings.ftp.has_source() {
        return Err("Спочатку вкажіть FTP / autoconfig URL у налаштуваннях".into());
    }
    if settings.mods_path.trim().is_empty() {
        return Ok(SyncStatus {
            ready: false,
            bytes_missing: 0,
            files_missing: 0,
        });
    }

    let repo = resolve_repo(&settings.ftp).await?;
    let mods_root = PathBuf::from(settings.resolved_mods_path());
    let mut bytes_missing = 0_u64;
    let mut files_missing = 0_u32;
    let mut stream = if repo.source.scheme == "ftp" && repo.inventory.is_none() {
        Some(connect(&repo).await?)
    } else {
        None
    };

    for mod_name in &settings.selected_mods {
        ensure_safe_name(mod_name)?;
        let remote = files_for_mod(&repo, stream.as_mut(), mod_name).await?;
        let needed = files_to_download(&mods_root.join(mod_name), &remote);
        files_missing += needed.len() as u32;
        bytes_missing += needed.iter().map(|f| f.size).sum::<u64>();
    }

    if let Some(mut stream) = stream {
        let _ = stream.quit().await;
    }
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
    if !settings.ftp.has_source() {
        return Err("Спочатку вкажіть FTP / autoconfig URL у налаштуваннях".into());
    }
    if settings.mods_path.trim().is_empty() {
        return Err("Спочатку вкажіть теку аддонів".into());
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
            message: Some("Сканування репозиторію...".into()),
        },
    );

    let repo = resolve_repo(&settings.ftp).await?;
    let mods_root = PathBuf::from(settings.resolved_mods_path());
    std::fs::create_dir_all(&mods_root)
        .map_err(|e| format!("Не вдалося створити теку модів: {e}"))?;

    let mut stream = if repo.source.scheme == "ftp" {
        Some(connect(&repo).await?)
    } else {
        None
    };

    let mut planned: Vec<(String, RemoteFile)> = Vec::new();
    for mod_name in &settings.selected_mods {
        check_cancel(cancel)?;
        ensure_safe_name(mod_name)?;
        let remote = files_for_mod(&repo, stream.as_mut(), mod_name).await?;
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
        if let Some(mut stream) = stream {
            let _ = stream.quit().await;
        }
        return Ok(());
    }

    let http = if repo.source.scheme != "ftp" {
        Some(http_client())
    } else {
        None
    };

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
        if let Some(stream) = stream.as_mut() {
            download_ftp_file(stream, &remote_path, &local_path, cancel, |chunk| {
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
        } else if let Some(client) = http.as_ref() {
            let url = a3s::join_remote_url(&repo.source, &remote_path)?;
            download_http_file(client, &url, &local_path, cancel, |chunk| {
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
        }

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
    if let Some(mut stream) = stream {
        let _ = stream.quit().await;
    }
    Ok(())
}

async fn resolve_repo(ftp: &FtpSettings) -> Result<ResolvedRepo, String> {
    let mut source = initial_source(ftp)?;
    source = overlay_settings(source, ftp);
    let mut inventory = None;
    let mut repository_name = None;
    let mut mode = if source.autoconfig { "a3s" } else { "ftp" };

    if source.autoconfig {
        let autoconfig_bytes = fetch_repo_file(&source, ftp, ".a3s/autoconfig").await?;
        let autoconfig = a3s::decode_autoconfig(&autoconfig_bytes)?;
        repository_name = Some(autoconfig.repository_name.clone()).filter(|name| !name.is_empty());
        source = overlay_settings(a3s::apply_protocol(&source, &autoconfig.protocol), ftp);
        match fetch_repo_file(&source, ftp, ".a3s/sync").await {
            Ok(bytes) => match a3s::decode_sync(&bytes) {
                Ok(parsed) => inventory = Some(parsed),
                Err(_) => mode = "a3s",
            },
            Err(_) => mode = "a3s",
        }
    }

    let username = if source.username.trim().is_empty() {
        if ftp.username.trim().is_empty() {
            "anonymous".into()
        } else {
            ftp.username.trim().to_string()
        }
    } else {
        source.username.clone()
    };
    let password = if ftp.username.trim().is_empty() {
        source.password.clone()
    } else {
        ftp.password.clone()
    };

    Ok(ResolvedRepo {
        source,
        username,
        password,
        passive: ftp.passive,
        mode,
        inventory,
        repository_name,
    })
}

fn initial_source(ftp: &FtpSettings) -> Result<SourceUrl, String> {
    if !ftp.source_url.trim().is_empty() {
        a3s::parse_source_url(ftp.source_url.trim())
    } else if !ftp.host.trim().is_empty() {
        Ok(SourceUrl {
            scheme: "ftp".into(),
            host: ftp.host.trim().to_string(),
            port: if ftp.port == 0 { 21 } else { ftp.port },
            username: ftp.username.clone(),
            password: ftp.password.clone(),
            remote_path: if ftp.remote_path.trim().is_empty() {
                "/".into()
            } else {
                ftp.remote_path.clone()
            },
            autoconfig: false,
        })
    } else {
        Err("Вкажіть FTP / autoconfig URL".into())
    }
}

fn overlay_settings(mut source: SourceUrl, ftp: &FtpSettings) -> SourceUrl {
    if !ftp.username.trim().is_empty() {
        source.username = ftp.username.trim().to_string();
        source.password = ftp.password.clone();
    }
    let scheme_default = a3s::default_port(&source.scheme);
    if ftp.port != 0 && ftp.port != scheme_default {
        source.port = ftp.port;
    }
    source
}

fn probe_from(repo: &ResolvedRepo) -> FtpProbe {
    FtpProbe {
        mode: repo.mode.into(),
        host: repo.source.host.clone(),
        port: repo.source.port,
        remote_path: repo.source.remote_path.clone(),
        has_sync: repo.inventory.is_some(),
        repository_name: repo.repository_name.clone(),
    }
}

async fn files_for_mod(
    repo: &ResolvedRepo,
    stream: Option<&mut AsyncFtpStream>,
    mod_name: &str,
) -> Result<Vec<RemoteFile>, String> {
    if let Some(inventory) = &repo.inventory {
        if let Some(addon) = inventory
            .addons
            .iter()
            .find(|addon| addon.name == mod_name)
        {
            return Ok(addon
                .files
                .iter()
                .map(|file| RemoteFile {
                    rel_path: file.rel_path.clone(),
                    size: file.size,
                    mtime: None,
                    sha1: file.sha1.clone(),
                })
                .collect());
        }
    }
    let stream = stream.ok_or_else(|| {
        format!("Немає маніфесту A3S для мода {mod_name}, а LIST недоступний")
    })?;
    walk_remote(stream, mod_name).await
}

async fn fetch_repo_file(
    source: &SourceUrl,
    ftp: &FtpSettings,
    relative: &str,
) -> Result<Vec<u8>, String> {
    if source.scheme == "ftp" {
        let repo = ResolvedRepo {
            source: source.clone(),
            username: if source.username.trim().is_empty() {
                if ftp.username.trim().is_empty() {
                    "anonymous".into()
                } else {
                    ftp.username.trim().to_string()
                }
            } else {
                source.username.clone()
            },
            password: if ftp.username.trim().is_empty() {
                source.password.clone()
            } else {
                ftp.password.clone()
            },
            passive: ftp.passive,
            mode: "a3s",
            inventory: None,
            repository_name: None,
        };
        let mut stream = connect(&repo).await?;
        let bytes = retr_bytes(&mut stream, relative).await;
        let _ = stream.quit().await;
        bytes
    } else {
        let url = a3s::join_remote_url(source, relative)?;
        let response = http_client()
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Не вдалося завантажити {relative}: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Не вдалося завантажити {relative}: HTTP {}",
                response.status()
            ));
        }
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|e| format!("Не вдалося прочитати {relative}: {e}"))
    }
}

async fn connect(repo: &ResolvedRepo) -> Result<AsyncFtpStream, String> {
    if repo.source.host.trim().is_empty() {
        return Err("Вкажіть адресу FTP".into());
    }
    let addr = format!("{}:{}", repo.source.host.trim(), repo.source.port);
    let mut stream = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| format!("Не вдалося підключитися до FTP ({addr}): {e}"))?;

    stream.set_mode(if repo.passive {
        Mode::Passive
    } else {
        Mode::Active
    });

    stream
        .login(&repo.username, &repo.password)
        .await
        .map_err(|e| format!("Помилка авторизації FTP: {e}"))?;
    stream
        .transfer_type(FileType::Binary)
        .await
        .map_err(|e| format!("Не вдалося увімкнути бінарний режим: {e}"))?;

    let remote = repo.source.remote_path.trim();
    if !remote.is_empty() && remote != "/" {
        stream
            .cwd(remote)
            .await
            .map_err(|e| format!("Не вдалося відкрити теку {remote}: {e}"))?;
    }
    Ok(stream)
}

async fn retr_bytes(stream: &mut AsyncFtpStream, path: &str) -> Result<Vec<u8>, String> {
    let mut reader = stream
        .retr_as_stream(path)
        .await
        .map_err(|e| format!("Не вдалося завантажити {path}: {e}"))?;
    let mut buf = Vec::new();
    let mut chunk = vec![0_u8; CHUNK];
    let mut read_err = None;
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) => {
                read_err = Some(format!("Помилка читання {path}: {e}"));
                break;
            }
        }
    }
    if let Err(err) = reader.finish().await {
        return Err(format!("Не вдалося завершити завантаження {path}: {err}"));
    }
    if let Some(err) = read_err {
        return Err(err);
    }
    Ok(buf)
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
        if !is_safe_name(&name) || is_metadata_dir(&name) {
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
                sha1: None,
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
    if let Some(remote_sha) = file.sha1.as_deref().filter(|value| !value.is_empty()) {
        return match file_sha1(&local) {
            Some(local_sha) => !local_sha.eq_ignore_ascii_case(remote_sha),
            None => true,
        };
    }
    if let (Some(remote_mtime), Ok(modified)) = (file.mtime, meta.modified()) {
        let local_mtime = DateTime::<Utc>::from(modified);
        if remote_mtime.timestamp() > local_mtime.timestamp() + 2 {
            return true;
        }
    }
    false
}

fn file_sha1(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    let mut buf = [0_u8; CHUNK];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    )
}

async fn download_ftp_file<F>(
    stream: &mut AsyncFtpStream,
    remote_path: &str,
    local_path: &Path,
    cancel: &AtomicBool,
    mut on_chunk: F,
) -> Result<(), String>
where
    F: FnMut(u64),
{
    prepare_local(local_path).await?;
    let tmp = temp_path(local_path);
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

    finish_temp(&tmp, local_path, download_err).await
}

async fn download_http_file<F>(
    client: &reqwest::Client,
    url: &str,
    local_path: &Path,
    cancel: &AtomicBool,
    mut on_chunk: F,
) -> Result<(), String>
where
    F: FnMut(u64),
{
    prepare_local(local_path).await?;
    let tmp = temp_path(local_path);
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Не вдалося завантажити {url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Не вдалося завантажити {url}: HTTP {}", response.status()));
    }
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("Не вдалося створити файл {}: {e}", tmp.display()))?;
    let mut download_err: Option<String> = None;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Помилка читання {url}: {e}"))?
    {
        if let Err(err) = check_cancel(cancel) {
            download_err = Some(err);
            break;
        }
        if let Err(e) = file.write_all(&chunk).await {
            download_err = Some(format!("Помилка запису {}: {e}", tmp.display()));
            break;
        }
        on_chunk(chunk.len() as u64);
    }
    if download_err.is_none() {
        if let Err(e) = file.flush().await {
            download_err = Some(format!("Не вдалося зберегти {}: {e}", tmp.display()));
        }
    }
    drop(file);
    finish_temp(&tmp, local_path, download_err).await
}

async fn prepare_local(local_path: &Path) -> Result<(), String> {
    if let Some(parent) = local_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Не вдалося створити теку {}: {e}", parent.display()))?;
    }
    Ok(())
}

fn temp_path(local_path: &Path) -> PathBuf {
    local_path.with_extension(format!(
        "{}part",
        local_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!("{e}."))
            .unwrap_or_default()
    ))
}

async fn finish_temp(
    tmp: &Path,
    local_path: &Path,
    download_err: Option<String>,
) -> Result<(), String> {
    if let Some(err) = download_err {
        let _ = tokio::fs::remove_file(tmp).await;
        return Err(err);
    }
    tokio::fs::rename(tmp, local_path)
        .await
        .map_err(|e| format!("Не вдалося перемістити файл {}: {e}", local_path.display()))
}

fn http_client() -> reqwest::Client {
    reqwest::Client::new()
}

fn file_name(file: &File) -> String {
    file.name().to_string()
}

fn is_dot(file: &File) -> bool {
    matches!(file.name(), "." | "..")
}

fn is_metadata_dir(name: &str) -> bool {
    name.eq_ignore_ascii_case(".a3s")
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
