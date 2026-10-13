use std::collections::BTreeMap;
use std::path::Path;

use base64::Engine;
use url::Url;

pub const PUBLIC_KEYS: [&str; 6] = [
    "MARS_MANIFEST_BASE_URL",
    "MARS_MANIFEST_PUBLIC_KEY",
    "MARS_COMMUNITY_API_BASE",
    "MARS_WEBSITE_BASE",
    "MARS_SPONSORS_URL",
    "MARS_UPDATER_PUBLIC_KEY",
];
pub const LEGACY_API_KEY: &str = "MARS_COMMAND_API_URL";

pub fn read_public_file(path: &Path, required: bool) -> Result<BTreeMap<String, String>, String> {
    if !path.exists() && !required {
        return Ok(BTreeMap::new());
    }
    let entries = dotenvy::from_path_iter(path)
        .map_err(|_| format!("Cannot read public build configuration: {}", path.display()))?;
    let mut values = BTreeMap::new();
    for entry in entries {
        let (key, value) = entry.map_err(|_| {
            format!(
                "Invalid dotenv syntax in {}. Configuration values are not logged.",
                path.display()
            )
        })?;
        if PUBLIC_KEYS.contains(&key.as_str()) || key == LEGACY_API_KEY {
            if values.insert(key.clone(), value).is_some() {
                return Err(format!(
                    "Duplicate public setting {key} in {}",
                    path.display()
                ));
            }
        } else if required {
            return Err(format!(
                "The versioned public configuration contains an unapproved setting: {key}"
            ));
        }
    }
    Ok(values)
}

fn normalize_layer(values: &BTreeMap<String, String>) -> Result<BTreeMap<String, String>, String> {
    let mut result: BTreeMap<_, _> = values
        .iter()
        .filter(|(key, _)| PUBLIC_KEYS.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.trim().to_string()))
        .collect();
    if !result.contains_key("MARS_MANIFEST_BASE_URL") {
        if let Some(value) = values.get(LEGACY_API_KEY) {
            let mut base = safe_base(value, LEGACY_API_KEY)?;
            match base.path().trim_end_matches('/') {
                "" => base.set_path("/api/v1"),
                "/api" => base.set_path("/api/v1"),
                "/api/v1" => base.set_path("/api/v1"),
                _ => {
                    return Err(
                        "MARS_COMMAND_API_URL must be the server origin or /api/v1 base; use MARS_MANIFEST_BASE_URL for custom paths."
                            .into(),
                    );
                }
            }
            result.insert("MARS_MANIFEST_BASE_URL".into(), base.to_string());
        }
    }
    Ok(result)
}

pub fn resolve_public_config(
    defaults: &BTreeMap<String, String>,
    local: &BTreeMap<String, String>,
    environment: &BTreeMap<String, String>,
    release: bool,
) -> Result<BTreeMap<String, String>, String> {
    let mut values = normalize_layer(defaults)?;
    values.extend(normalize_layer(local)?);
    values.extend(normalize_layer(environment)?);
    for key in PUBLIC_KEYS {
        let value = values.get(key).map(String::as_str).unwrap_or("");
        if value.chars().any(|character| character.is_control()) {
            return Err(format!("{key} must not contain control characters."));
        }
        match key {
            "MARS_MANIFEST_PUBLIC_KEY" => {
                if value.len() != 64
                    || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
                    || value.bytes().all(|byte| byte == b'0')
                {
                    return Err("MARS_MANIFEST_PUBLIC_KEY must be a nonzero 32-byte Ed25519 public key in hex.".into());
                }
            }
            "MARS_UPDATER_PUBLIC_KEY" => {
                if value.is_empty() && !release {
                    continue;
                }
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(value)
                    .map_err(
                        |_| "MARS_UPDATER_PUBLIC_KEY must contain a Tauri public key, not a path.",
                    )?;
                let decoded = std::str::from_utf8(&bytes)
                    .map_err(|_| "MARS_UPDATER_PUBLIC_KEY is not valid public-key text.")?;
                minisign_verify::PublicKey::decode(decoded)
                    .map_err(|_| "MARS_UPDATER_PUBLIC_KEY is not a valid Minisign public key.")?;
            }
            "MARS_SPONSORS_URL" => {
                if value.is_empty() {
                    continue;
                }
                let url = safe_base(value, key)?;
                let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
                if url.scheme() != "https"
                    || url.host_str() != Some("github.com")
                    || url.port_or_known_default() != Some(443)
                    || parts.len() != 2
                    || parts[0] != "sponsors"
                    || parts[1].is_empty()
                {
                    return Err(
                        "MARS_SPONSORS_URL must identify an HTTPS GitHub Sponsors recipient."
                            .into(),
                    );
                }
            }
            _ => {
                if value.is_empty() && !release && key != "MARS_MANIFEST_BASE_URL" {
                    continue;
                }
                let url = safe_base(value, key)?;
                if key == "MARS_COMMUNITY_API_BASE"
                    && url.path().trim_end_matches('/').ends_with("/api")
                {
                    return Err("MARS_COMMUNITY_API_BASE must not include the /api suffix.".into());
                }
            }
        }
    }
    Ok(values)
}

fn safe_base(value: &str, name: &str) -> Result<Url, String> {
    let url = Url::parse(value.trim()).map_err(|_| format!("{name} must be a valid URL."))?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if !(url.scheme() == "https" || (url.scheme() == "http" && local))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(format!(
            "{name} requires HTTPS (HTTP only for loopback), without credentials, query or fragment."
        ));
    }
    Ok(url)
}
