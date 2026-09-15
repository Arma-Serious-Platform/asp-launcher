use std::io::{Cursor, Read};

use flate2::read::GzDecoder;
use jaded::{Content, Parser, PrimitiveType, Value};
use url::Url;

const JAVA_MAGIC: [u8; 2] = [0xac, 0xed];
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

#[derive(Debug, Clone)]
pub struct SourceUrl {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub remote_path: String,
    pub autoconfig: bool,
}

#[derive(Debug, Clone)]
pub struct AutoConfig {
    pub repository_name: String,
    pub protocol: Protocol,
}

#[derive(Debug, Clone)]
pub struct Protocol {
    pub url: String,
    pub port: u16,
    pub login: String,
    pub password: String,
    pub protocol_type: String,
}

#[derive(Debug, Clone, Default)]
pub struct SyncFile {
    pub rel_path: String,
    pub size: u64,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SyncAddon {
    pub name: String,
    pub files: Vec<SyncFile>,
}

#[derive(Debug, Clone, Default)]
pub struct SyncInventory {
    pub addons: Vec<SyncAddon>,
}

impl SyncInventory {
    pub fn remote_bytes(&self, name: &str) -> u64 {
        self.addons
            .iter()
            .find(|addon| addon.name == name)
            .map(|addon| addon.files.iter().map(|file| file.size).sum())
            .unwrap_or(0)
    }
}

pub fn default_port(scheme: &str) -> u16 {
    match scheme {
        "https" => 443,
        "http" => 80,
        _ => 21,
    }
}

pub fn parse_source_url(raw: &str) -> Result<SourceUrl, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Вкажіть FTP / autoconfig URL".into());
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("ftp://{trimmed}")
    };
    let parsed = Url::parse(&with_scheme).map_err(|e| format!("Некоректний URL: {e}"))?;
    let scheme = parsed.scheme().to_ascii_lowercase();
    if !matches!(scheme.as_str(), "ftp" | "http" | "https") {
        return Err(format!("Непідтримувана схема {scheme}"));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "У URL немає хоста".to_string())?
        .to_string();
    let port = parsed.port().unwrap_or_else(|| default_port(&scheme));
    let mut path = parsed.path().to_string();
    if path.is_empty() {
        path = "/".into();
    }
    let autoconfig = is_autoconfig_path(&path);
    if autoconfig {
        path = strip_autoconfig_suffix(&path);
    }
    if path.is_empty() {
        path = "/".into();
    }
    Ok(SourceUrl {
        scheme,
        host,
        port,
        username: parsed.username().to_string(),
        password: parsed.password().unwrap_or("").to_string(),
        remote_path: path,
        autoconfig,
    })
}

pub fn decode_autoconfig(bytes: &[u8]) -> Result<AutoConfig, String> {
    let value = parse_java(bytes)?;
    let obj = value
        .object_data()
        .ok_or_else(|| "autoconfig не є Java-об'єктом".to_string())?;
    let protocol_value = obj
        .get_field("protocole")
        .ok_or_else(|| "У autoconfig немає поля protocole".to_string())?;
    Ok(AutoConfig {
        repository_name: field_string(obj, "repositoryName").unwrap_or_default(),
        protocol: decode_protocol(protocol_value)?,
    })
}

pub fn decode_sync(bytes: &[u8]) -> Result<SyncInventory, String> {
    let value = parse_java(bytes)?;
    let root = value
        .object_data()
        .ok_or_else(|| "sync не є Java-об'єктом".to_string())?;
    let children = child_directories(root);
    let marked: Vec<_> = children
        .iter()
        .copied()
        .filter(|dir| field_bool(dir, "markAsAddon"))
        .collect();
    let addons_src = if marked.is_empty() { children } else { marked };
    let mut addons = Vec::new();
    for dir in addons_src {
        let name = field_string(dir, "name").unwrap_or_default();
        if name.is_empty() || name == ".a3s" || !is_safe_segment(&name) {
            continue;
        }
        if field_bool(dir, "deleted") || field_bool(dir, "hidden") {
            continue;
        }
        let mut files = Vec::new();
        collect_files(dir, "", &mut files);
        addons.push(SyncAddon { name, files });
    }
    addons.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(SyncInventory { addons })
}

pub fn apply_protocol(source: &SourceUrl, protocol: &Protocol) -> SourceUrl {
    let scheme = protocol_scheme(&protocol.protocol_type, &source.scheme);
    let (host, path) = split_protocol_url(&protocol.url, &source.host);
    let port = if protocol.port == 0 {
        default_port(&scheme)
    } else {
        protocol.port
    };
    SourceUrl {
        scheme,
        host,
        port,
        username: if protocol.login.trim().is_empty() {
            source.username.clone()
        } else {
            protocol.login.clone()
        },
        password: protocol.password.clone(),
        remote_path: if path.is_empty() {
            source.remote_path.clone()
        } else {
            path
        },
        autoconfig: true,
    }
}

pub fn join_remote_url(source: &SourceUrl, relative: &str) -> Result<String, String> {
    let mut url = Url::parse(&format!(
        "{}://{}:{}",
        source.scheme, source.host, source.port
    ))
    .map_err(|e| format!("Некоректний URL репозиторію: {e}"))?;
    let mut segments = Vec::new();
    for part in source
        .remote_path
        .split('/')
        .chain(relative.split('/'))
        .filter(|part| !part.is_empty())
    {
        segments.push(part.to_string());
    }
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| "Не вдалося зібрати HTTP-шлях".to_string())?;
        path.clear();
        for part in &segments {
            path.push(part);
        }
    }
    Ok(url.to_string())
}

fn decode_protocol(value: &Value) -> Result<Protocol, String> {
    let obj = value
        .object_data()
        .ok_or_else(|| "protocole не є об'єктом".to_string())?;
    let protocol_type = obj
        .get_field("protocolType")
        .and_then(enum_or_name)
        .unwrap_or_else(|| "FTP".into());
    let port_raw = field_string(obj, "port").unwrap_or_default();
    let port = port_raw.parse::<u16>().unwrap_or_else(|_| default_port(&protocol_scheme(&protocol_type, "ftp")));
    Ok(Protocol {
        url: field_string(obj, "url").unwrap_or_default(),
        port,
        login: field_string(obj, "login").unwrap_or_default(),
        password: field_string(obj, "password").unwrap_or_default(),
        protocol_type,
    })
}

fn protocol_scheme(protocol_type: &str, fallback: &str) -> String {
    let name = protocol_type.to_ascii_uppercase();
    if name.contains("HTTPS") {
        "https".into()
    } else if name.contains("HTTP") {
        "http".into()
    } else if name.contains("FTP") {
        "ftp".into()
    } else {
        fallback.to_ascii_lowercase()
    }
}

fn split_protocol_url(url: &str, fallback_host: &str) -> (String, String) {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return (fallback_host.to_string(), "/".into());
    }
    if trimmed.contains("://") {
        if let Ok(parsed) = parse_source_url(trimmed) {
            return (parsed.host, parsed.remote_path);
        }
    }
    let stripped = strip_autoconfig_suffix(trimmed);
    let stripped = stripped.trim_start_matches('/');
    if let Some((host, rest)) = stripped.split_once('/') {
        if host.contains('.') || host.chars().all(|c| c.is_ascii_digit() || c == '.') {
            let path = if rest.is_empty() {
                "/".into()
            } else {
                format!("/{rest}")
            };
            return (host.to_string(), path);
        }
    }
    if stripped.contains('.') && !stripped.contains('/') {
        return (stripped.to_string(), "/".into());
    }
    (
        fallback_host.to_string(),
        if stripped.is_empty() {
            "/".into()
        } else {
            format!("/{stripped}")
        },
    )
}

fn parse_java(bytes: &[u8]) -> Result<Value, String> {
    let decoded = maybe_gunzip(bytes)?;
    if decoded.len() < 2 || decoded[0] != JAVA_MAGIC[0] || decoded[1] != JAVA_MAGIC[1] {
        return Err("Файл A3S не є Java-серіалізацією".into());
    }
    let mut parser =
        Parser::new(Cursor::new(decoded)).map_err(|e| format!("Не вдалося відкрити A3S: {e}"))?;
    match parser
        .read()
        .map_err(|e| format!("Не вдалося розібрати A3S: {e}"))?
    {
        Content::Object(value) => Ok(value),
        Content::Block(_) => Err("Очікувався Java-об'єкт A3S".into()),
    }
}

fn maybe_gunzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() >= 2 && bytes[0] == GZIP_MAGIC[0] && bytes[1] == GZIP_MAGIC[1] {
        let mut decoder = GzDecoder::new(bytes);
        let mut out = Vec::new();
        decoder
            .read_to_end(&mut out)
            .map_err(|e| format!("Не вдалося розпакувати gzip A3S: {e}"))?;
        return Ok(out);
    }
    Ok(bytes.to_vec())
}

fn child_directories(dir: &jaded::ObjectData) -> Vec<&jaded::ObjectData> {
    list_values(dir.get_field("list"))
        .into_iter()
        .filter_map(|value| value.object_data())
        .filter(|child| child.class_name().contains("SyncTreeDirectory"))
        .collect()
}

fn collect_files(dir: &jaded::ObjectData, rel: &str, out: &mut Vec<SyncFile>) {
    for child in list_values(dir.get_field("list")) {
        match child {
            Value::Object(data) => {
                let name = field_string(data, "name").unwrap_or_default();
                if name.is_empty() || name == ".a3s" || !is_safe_segment(&name) {
                    continue;
                }
                if field_bool(data, "deleted") {
                    continue;
                }
                let child_rel = if rel.is_empty() {
                    name.clone()
                } else {
                    format!("{rel}/{name}")
                };
                if data.class_name().contains("SyncTreeLeaf") {
                    let sha1 = field_string(data, "sha1").filter(|value| !value.is_empty());
                    out.push(SyncFile {
                        rel_path: child_rel,
                        size: field_long(data, "size").max(0) as u64,
                        sha1,
                    });
                } else if data.class_name().contains("SyncTreeDirectory") {
                    if field_bool(data, "hidden") {
                        continue;
                    }
                    collect_files(data, &child_rel, out);
                }
            }
            Value::Loop(_) | Value::Null => {}
            _ => {}
        }
    }
}

fn list_values(value: Option<&Value>) -> Vec<&Value> {
    let Some(value) = value else {
        return Vec::new();
    };
    if let Some(items) = value.array() {
        return items.iter().collect();
    }
    let Some(obj) = value.object_data() else {
        return Vec::new();
    };
    if let Some(items) = obj.get_field("elementData").and_then(|field| field.array()) {
        let size = field_long(obj, "size").max(0) as usize;
        return items.iter().take(size).collect();
    }
    if let Some(mut anno) = obj.get_annotation(0) {
        if let Ok(count) = anno.read_i32() {
            let mut items = Vec::with_capacity(count.max(0) as usize);
            for _ in 0..count.max(0) {
                if let Ok(item) = anno.read_object() {
                    items.push(item);
                }
            }
            return items;
        }
    }
    Vec::new()
}

fn field_string(obj: &jaded::ObjectData, name: &str) -> Option<String> {
    obj.get_field(name).and_then(value_string)
}

fn field_bool(obj: &jaded::ObjectData, name: &str) -> bool {
    obj.get_field(name)
        .and_then(value_bool)
        .unwrap_or(false)
}

fn field_long(obj: &jaded::ObjectData, name: &str) -> i64 {
    obj.get_field(name).and_then(value_long).unwrap_or(0)
}

fn value_string(value: &Value) -> Option<String> {
    if let Some(text) = value.string() {
        return Some(text.to_string());
    }
    if let Some((.., name)) = value.enum_data() {
        return Some(name.to_string());
    }
    if let Some(obj) = value.object_data() {
        return field_string(obj, "value").or_else(|| field_string(obj, "name"));
    }
    None
}

fn value_bool(value: &Value) -> Option<bool> {
    match value.primitive() {
        Some(PrimitiveType::Boolean(flag)) => Some(*flag),
        _ => value.object_data().and_then(|obj| {
            obj.get_field("value").and_then(value_bool)
        }),
    }
}

fn value_long(value: &Value) -> Option<i64> {
    match value.primitive() {
        Some(PrimitiveType::Long(num)) => Some(*num),
        Some(PrimitiveType::Int(num)) => Some(*num as i64),
        Some(PrimitiveType::Short(num)) => Some(*num as i64),
        _ => value.object_data().and_then(|obj| {
            obj.get_field("value").and_then(value_long)
        }),
    }
}

fn enum_or_name(value: &Value) -> Option<String> {
    if let Some((_, name)) = value.enum_data() {
        return Some(name.to_string());
    }
    if let Some(obj) = value.object_data() {
        return field_string(obj, "name")
            .or_else(|| field_string(obj, "protocol"))
            .or_else(|| field_string(obj, "description"));
    }
    value_string(value)
}

fn is_autoconfig_path(path: &str) -> bool {
    path.trim_end_matches('/')
        .to_ascii_lowercase()
        .ends_with(".a3s/autoconfig")
}

fn strip_autoconfig_suffix(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    let lower = trimmed.to_ascii_lowercase();
    if let Some(idx) = lower.rfind("/.a3s/autoconfig") {
        let root = &trimmed[..idx];
        if root.is_empty() {
            "/".into()
        } else {
            root.to_string()
        }
    } else {
        trimmed.to_string()
    }
}

fn is_safe_segment(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wog_autoconfig_url() {
        let parsed = parse_source_url("ftp://88.99.69.243/Repo/WOG/.a3s/autoconfig").unwrap();
        assert_eq!(parsed.host, "88.99.69.243");
        assert_eq!(parsed.port, 21);
        assert_eq!(parsed.remote_path, "/Repo/WOG");
        assert!(parsed.autoconfig);
    }

    #[test]
    fn parses_simple_ftp_url() {
        let parsed = parse_source_url("ftp://example.com/mods").unwrap();
        assert!(!parsed.autoconfig);
        assert_eq!(parsed.remote_path, "/mods");
    }
}
