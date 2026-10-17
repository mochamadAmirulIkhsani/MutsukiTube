use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_MANIFEST_BYTES: usize = 1 << 20;

// Development fallback key only. Production builds should override this
// through MUTSUKITUBE_PLUGIN_PUBLIC_KEY or a build-time trusted key.
pub const DEVELOPMENT_PLUGIN_PUBLIC_KEY: &str =
    "e87eb6823b81521e6986c504641f1b60ef5f73a5d92abbfad453f6e1cb5315a0";

pub fn trusted_public_key() -> Option<String> {
    std::env::var("MUTSUKITUBE_PLUGIN_PUBLIC_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| option_env!("MUTSUKITUBE_PLUGIN_PUBLIC_KEY").map(str::to_string))
        .or_else(|| {
            if cfg!(debug_assertions) {
                Some(DEVELOPMENT_PLUGIN_PUBLIC_KEY.to_string())
            } else {
                None
            }
        })
}
pub const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub schema: String,
    pub version: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    #[serde(rename = "urlBase", default)]
    pub url_base: String,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    pub path: String,
    pub provider: String,
    pub os: String,
    pub arch: String,
    pub sha256: String,
    pub size: i64,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginState {
    pub active_version: Option<String>,
    pub previous_version: Option<String>,
    pub installed_versions: Vec<String>,
    pub failed_versions: Vec<String>,
    pub last_check_unix: Option<u64>,
}

impl Default for PluginState {
    fn default() -> Self {
        Self {
            active_version: None,
            previous_version: None,
            installed_versions: Vec::new(),
            failed_versions: Vec::new(),
            last_check_unix: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdaterError {
    InvalidSignature,
    InvalidPublicKey,
    InvalidManifest(String),
    IncompatiblePlatform(String),
    UnsafePath(String),
    UnsafeUrl(String),
    ChecksumMismatch { expected: String, actual: String },
    SizeMismatch { expected: i64, actual: usize },
    TooLarge { limit: usize, actual: usize },
    Io(String),
    Network(String),
    AntiRollback { current: String, candidate: String },
}

impl std::fmt::Display for UpdaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "invalid manifest signature"),
            Self::InvalidPublicKey => write!(f, "invalid public key"),
            Self::InvalidManifest(msg) => write!(f, "invalid manifest: {msg}"),
            Self::IncompatiblePlatform(msg) => write!(f, "incompatible platform: {msg}"),
            Self::UnsafePath(msg) => write!(f, "unsafe artifact path: {msg}"),
            Self::UnsafeUrl(msg) => write!(f, "unsafe artifact URL: {msg}"),
            Self::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected}, got {actual}")
            }
            Self::SizeMismatch { expected, actual } => {
                write!(f, "size mismatch: expected {expected}, got {actual}")
            }
            Self::TooLarge { limit, actual } => {
                write!(f, "artifact too large: limit {limit}, got {actual}")
            }
            Self::Io(msg) => write!(f, "io error: {msg}"),
            Self::Network(msg) => write!(f, "network error: {msg}"),
            Self::AntiRollback { current, candidate } => {
                write!(f, "anti-rollback rejected {candidate} after {current}")
            }
        }
    }
}

impl std::error::Error for UpdaterError {}

pub fn verify_manifest(
    manifest_bytes: &[u8],
    signature_hex: &str,
    public_key_hex: &str,
) -> Result<Manifest, UpdaterError> {
    if manifest_bytes.is_empty() || manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return Err(UpdaterError::InvalidManifest(
            "unexpected manifest size".into(),
        ));
    }

    let public_key_bytes =
        hex::decode(public_key_hex).map_err(|_| UpdaterError::InvalidPublicKey)?;
    if public_key_bytes.len() != 32 {
        return Err(UpdaterError::InvalidPublicKey);
    }
    let verifying_key = VerifyingKey::from_bytes(
        public_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| UpdaterError::InvalidPublicKey)?,
    )
    .map_err(|_| UpdaterError::InvalidPublicKey)?;

    let signature_bytes =
        hex::decode(signature_hex.trim()).map_err(|_| UpdaterError::InvalidSignature)?;
    if signature_bytes.len() != 64 {
        return Err(UpdaterError::InvalidSignature);
    }
    let signature = Signature::from_bytes(
        signature_bytes
            .as_slice()
            .try_into()
            .map_err(|_| UpdaterError::InvalidSignature)?,
    );

    verifying_key
        .verify(manifest_bytes, &signature)
        .map_err(|_| UpdaterError::InvalidSignature)?;

    let manifest: Manifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| UpdaterError::InvalidManifest(error.to_string()))?;

    if manifest.schema != "mutsukitube-manifest-1" {
        return Err(UpdaterError::InvalidManifest(format!(
            "unsupported schema {}",
            manifest.schema
        )));
    }
    if manifest.version.trim().is_empty() {
        return Err(UpdaterError::InvalidManifest("missing version".into()));
    }
    for artifact in &manifest.artifacts {
        validate_artifact(artifact)?;
    }

    Ok(manifest)
}

fn validate_artifact(artifact: &Artifact) -> Result<(), UpdaterError> {
    if artifact.provider.trim().is_empty() {
        return Err(UpdaterError::InvalidManifest(
            "artifact missing provider".into(),
        ));
    }
    if artifact.os != "windows" && artifact.os != "linux" {
        return Err(UpdaterError::InvalidManifest(format!(
            "unsupported os {}",
            artifact.os
        )));
    }
    if artifact.arch != "amd64" && artifact.arch != "arm64" {
        return Err(UpdaterError::InvalidManifest(format!(
            "unsupported arch {}",
            artifact.arch
        )));
    }
    if artifact.sha256.len() != 64 || hex::decode(&artifact.sha256).is_err() {
        return Err(UpdaterError::InvalidManifest("invalid sha256".into()));
    }
    if artifact.size < 0 || artifact.size as usize > MAX_ARTIFACT_BYTES {
        return Err(UpdaterError::InvalidManifest(
            "invalid artifact size".into(),
        ));
    }
    validate_relative_path(&artifact.path)?;
    if !artifact.url.is_empty() {
        validate_url(&artifact.url)?;
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), UpdaterError> {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') {
        return Err(UpdaterError::UnsafePath(path.to_string()));
    }
    if path.contains("..") || path.contains('\\') {
        return Err(UpdaterError::UnsafePath(path.to_string()));
    }
    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if file_name.is_empty() || file_name != path {
        // Allow simple nested release paths, but reject traversal and drive letters.
        if path.contains(':') {
            return Err(UpdaterError::UnsafePath(path.to_string()));
        }
    }
    Ok(())
}

fn validate_url(url: &str) -> Result<(), UpdaterError> {
    if url.starts_with("https://") {
        return Ok(());
    }
    if url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost") {
        return Ok(());
    }
    Err(UpdaterError::UnsafeUrl(url.to_string()))
}

pub fn select_artifacts<'a>(manifest: &'a Manifest, os: &str, arch: &str) -> Vec<&'a Artifact> {
    manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.os == os && artifact.arch == arch)
        .collect()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn verify_artifact_bytes(artifact: &Artifact, bytes: &[u8]) -> Result<(), UpdaterError> {
    if artifact.size != 0 && artifact.size as usize != bytes.len() {
        return Err(UpdaterError::SizeMismatch {
            expected: artifact.size,
            actual: bytes.len(),
        });
    }
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(UpdaterError::TooLarge {
            limit: MAX_ARTIFACT_BYTES,
            actual: bytes.len(),
        });
    }
    let actual = sha256_hex(bytes);
    if actual != artifact.sha256 {
        return Err(UpdaterError::ChecksumMismatch {
            expected: artifact.sha256.clone(),
            actual,
        });
    }
    Ok(())
}

pub fn accepts_version(current: Option<&str>, candidate: &str) -> Result<(), UpdaterError> {
    let Some(current) = current else {
        return Ok(());
    };
    if compare_versions(candidate, current) >= 0 {
        Ok(())
    } else {
        Err(UpdaterError::AntiRollback {
            current: current.to_string(),
            candidate: candidate.to_string(),
        })
    }
}

fn compare_versions(left: &str, right: &str) -> i32 {
    let left = left.trim_start_matches('v');
    let right = right.trim_start_matches('v');
    let left_parts: Vec<i64> = left
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    let right_parts: Vec<i64> = right
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    let max = left_parts.len().max(right_parts.len());
    for index in 0..max {
        let l = left_parts.get(index).copied().unwrap_or(0);
        let r = right_parts.get(index).copied().unwrap_or(0);
        if l != r {
            return if l > r { 1 } else { -1 };
        }
    }
    0
}

pub fn plugin_base_dir() -> PathBuf {
    if let Ok(value) = std::env::var("MUTSUKITUBE_ANIME_PLUGIN_DIR") {
        return PathBuf::from(value);
    }
    if let Some(base) = dirs::data_local_dir() {
        return base.join("MutsukiTube").join("plugins");
    }
    PathBuf::from("plugins")
}

pub fn state_path(base_dir: &Path) -> PathBuf {
    base_dir.join("plugin-state.json")
}

pub fn load_state(base_dir: &Path) -> PluginState {
    let path = state_path(base_dir);
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_state(base_dir: &Path, state: &PluginState) -> Result<(), UpdaterError> {
    fs::create_dir_all(base_dir).map_err(|error| UpdaterError::Io(error.to_string()))?;
    let data =
        serde_json::to_vec_pretty(state).map_err(|error| UpdaterError::Io(error.to_string()))?;
    fs::write(state_path(base_dir), data).map_err(|error| UpdaterError::Io(error.to_string()))
}

pub fn install_artifact(
    base_dir: &Path,
    version: &str,
    artifact: &Artifact,
    bytes: &[u8],
) -> Result<PathBuf, UpdaterError> {
    accepts_version(None, version)?;
    verify_artifact_bytes(artifact, bytes)?;

    let version_dir = base_dir.join(version);
    fs::create_dir_all(&version_dir).map_err(|error| UpdaterError::Io(error.to_string()))?;

    let file_name = Path::new(&artifact.path)
        .file_name()
        .ok_or_else(|| UpdaterError::UnsafePath(artifact.path.clone()))?;
    let destination = version_dir.join(file_name);

    let temp_path = version_dir.join(format!(".{}.tmp", file_name.to_string_lossy()));
    fs::write(&temp_path, bytes).map_err(|error| UpdaterError::Io(error.to_string()))?;
    if destination.exists() {
        let _ = fs::remove_file(&destination);
    }
    fs::rename(&temp_path, &destination).map_err(|error| UpdaterError::Io(error.to_string()))?;

    Ok(destination)
}

pub async fn download_artifact(
    client: &reqwest::Client,
    url: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, UpdaterError> {
    validate_url(url)?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| UpdaterError::Network(error.to_string()))?;
    if !response.status().is_success() {
        return Err(UpdaterError::Network(format!("HTTP {}", response.status())));
    }
    if let Some(length) = response.content_length() {
        if length as usize > max_bytes {
            return Err(UpdaterError::TooLarge {
                limit: max_bytes,
                actual: length as usize,
            });
        }
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| UpdaterError::Network(error.to_string()))?;
    if bytes.len() > max_bytes {
        return Err(UpdaterError::TooLarge {
            limit: max_bytes,
            actual: bytes.len(),
        });
    }
    Ok(bytes.to_vec())
}

pub fn artifact_download_url(
    manifest: &Manifest,
    artifact: &Artifact,
) -> Result<String, UpdaterError> {
    if !artifact.url.is_empty() {
        validate_url(&artifact.url)?;
        return Ok(artifact.url.clone());
    }
    if !manifest.url_base.is_empty() {
        validate_url(&manifest.url_base)?;
        let base = manifest.url_base.trim_end_matches('/');
        return Ok(format!("{}/{}", base, artifact.path));
    }
    Err(UpdaterError::UnsafeUrl("missing artifact URL".into()))
}

pub fn mark_version_failed(base_dir: &Path, version: &str) -> Result<(), UpdaterError> {
    let mut state = load_state(base_dir);
    if !state.failed_versions.iter().any(|item| item == version) {
        state.failed_versions.push(version.to_string());
    }
    if state.active_version.as_deref() == Some(version) {
        state.active_version = state.previous_version.clone();
        state.previous_version = None;
    }
    state.last_check_unix = Some(now_unix());
    save_state(base_dir, &state)
}

pub fn is_version_failed(base_dir: &Path, version: &str) -> bool {
    load_state(base_dir)
        .failed_versions
        .iter()
        .any(|item| item == version)
}

pub fn rollback_to_previous(base_dir: &Path) -> Result<Option<String>, UpdaterError> {
    let mut state = load_state(base_dir);
    if let Some(previous) = state.previous_version.clone() {
        state.active_version = Some(previous.clone());
        state.previous_version = None;
        state.last_check_unix = Some(now_unix());
        save_state(base_dir, &state)?;
        return Ok(Some(previous));
    }
    Ok(state.active_version)
}

pub fn active_version(base_dir: &Path) -> Option<String> {
    load_state(base_dir).active_version
}

pub fn activate_version(base_dir: &Path, version: &str) -> Result<(), UpdaterError> {
    let mut state = load_state(base_dir);
    if let Some(current) = state.active_version.clone() {
        accepts_version(Some(&current), version)?;
        if current != version {
            state.previous_version = Some(current);
        }
    }
    if !state.installed_versions.iter().any(|item| item == version) {
        state.installed_versions.push(version.to_string());
    }
    state.active_version = Some(version.to_string());
    state.last_check_unix = Some(now_unix());
    save_state(base_dir, &state)
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            schema: "mutsukitube-manifest-1".into(),
            version: "v0.1.0".into(),
            generated_at: "2026-10-18T00:00:00Z".into(),
            url_base: "https://example.com/plugins/v0.1.0".into(),
            artifacts: vec![Artifact {
                path: "ylnime-windows-amd64.exe".into(),
                provider: "ylnime".into(),
                os: "windows".into(),
                arch: "amd64".into(),
                sha256: sha256_hex(b"hello"),
                size: 5,
                url: "https://example.com/plugins/v0.1.0/ylnime-windows-amd64.exe".into(),
            }],
        }
    }

    #[test]
    fn accepts_valid_manifest_signature() {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key = hex::encode(signing_key.verifying_key().to_bytes());
        let data = serde_json::to_vec(&manifest()).unwrap();
        let signature = signing_key.sign(&data);
        let signature_hex = hex::encode(signature.to_bytes());

        let verified = verify_manifest(&data, &signature_hex, &public_key).unwrap();
        assert_eq!(verified.version, "v0.1.0");
    }

    #[test]
    fn rejects_tampered_manifest() {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key = hex::encode(signing_key.verifying_key().to_bytes());
        let mut data = serde_json::to_vec(&manifest()).unwrap();
        data[10] ^= 1;
        let signature = signing_key.sign(&serde_json::to_vec(&manifest()).unwrap());
        let signature_hex = hex::encode(signature.to_bytes());

        assert!(verify_manifest(&data, &signature_hex, &public_key).is_err());
    }

    #[test]
    fn rejects_invalid_signature() {
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let other_key = SigningKey::from_bytes(&[9u8; 32]);
        let public_key = hex::encode(signing_key.verifying_key().to_bytes());
        let data = serde_json::to_vec(&manifest()).unwrap();
        let signature = other_key.sign(&data);
        let signature_hex = hex::encode(signature.to_bytes());

        assert!(verify_manifest(&data, &signature_hex, &public_key).is_err());
    }

    #[test]
    fn verifies_artifact_checksum() {
        let artifact = &manifest().artifacts[0];
        verify_artifact_bytes(artifact, b"hello").unwrap();
        assert!(verify_artifact_bytes(artifact, b"world").is_err());
    }

    #[test]
    fn rejects_anti_rollback() {
        assert!(accepts_version(Some("v0.2.0"), "v0.1.0").is_err());
        assert!(accepts_version(Some("v0.1.0"), "v0.1.0").is_ok());
        assert!(accepts_version(Some("v0.1.0"), "v0.2.0").is_ok());
    }

    #[test]
    fn installs_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let artifact = &manifest().artifacts[0];
        let path = install_artifact(temp.path(), "v0.1.0", artifact, b"hello").unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(path).unwrap(), b"hello");
    }

    #[test]
    fn activates_version() {
        let temp = tempfile::tempdir().unwrap();
        activate_version(temp.path(), "v0.1.0").unwrap();
        let state = load_state(temp.path());
        assert_eq!(state.active_version.as_deref(), Some("v0.1.0"));
    }

    #[test]
    fn activation_preserves_previous_known_good() {
        let temp = tempfile::tempdir().unwrap();
        activate_version(temp.path(), "v0.1.0").unwrap();
        activate_version(temp.path(), "v0.2.0").unwrap();

        let state = load_state(temp.path());
        assert_eq!(state.active_version.as_deref(), Some("v0.2.0"));
        assert_eq!(state.previous_version.as_deref(), Some("v0.1.0"));
        assert!(state.installed_versions.contains(&"v0.1.0".to_string()));
        assert!(state.installed_versions.contains(&"v0.2.0".to_string()));
    }

    #[test]
    fn rollback_restores_previous_version() {
        let temp = tempfile::tempdir().unwrap();
        activate_version(temp.path(), "v0.1.0").unwrap();
        activate_version(temp.path(), "v0.2.0").unwrap();

        let restored = rollback_to_previous(temp.path()).unwrap();
        assert_eq!(restored.as_deref(), Some("v0.1.0"));

        let state = load_state(temp.path());
        assert_eq!(state.active_version.as_deref(), Some("v0.1.0"));
        assert_eq!(state.previous_version, None);
    }

    #[test]
    fn mark_failed_rolls_back_and_blocks_repeat() {
        let temp = tempfile::tempdir().unwrap();
        activate_version(temp.path(), "v0.1.0").unwrap();
        activate_version(temp.path(), "v0.2.0").unwrap();

        mark_version_failed(temp.path(), "v0.2.0").unwrap();

        let state = load_state(temp.path());
        assert_eq!(state.active_version.as_deref(), Some("v0.1.0"));
        assert!(state.failed_versions.contains(&"v0.2.0".to_string()));
        assert!(is_version_failed(temp.path(), "v0.2.0"));
        assert!(!is_version_failed(temp.path(), "v0.1.0"));
    }

    #[test]
    fn interrupted_install_keeps_old_version_active() {
        let temp = tempfile::tempdir().unwrap();
        activate_version(temp.path(), "v0.1.0").unwrap();

        // Simulate a partially written payload without activation.
        let version_dir = temp.path().join("v0.2.0");
        std::fs::create_dir_all(&version_dir).unwrap();
        std::fs::write(
            version_dir.join(".ylnime-windows-amd64.exe.tmp"),
            b"partial",
        )
        .unwrap();

        let state = load_state(temp.path());
        assert_eq!(state.active_version.as_deref(), Some("v0.1.0"));
        assert!(!state.installed_versions.contains(&"v0.2.0".to_string()));
    }

    #[test]
    fn repeated_known_bad_release_is_detectable() {
        let temp = tempfile::tempdir().unwrap();
        activate_version(temp.path(), "v0.1.0").unwrap();
        activate_version(temp.path(), "v0.2.0").unwrap();
        mark_version_failed(temp.path(), "v0.2.0").unwrap();

        // A second activation of the same bad release must be preventable by
        // the caller checking is_version_failed before activating again.
        assert!(is_version_failed(temp.path(), "v0.2.0"));
    }

    #[tokio::test]
    async fn local_http_install_flow() {
        use ed25519_dalek::{Signer, SigningKey};
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key = hex::encode(signing_key.verifying_key().to_bytes());

        let temp = tempfile::tempdir().unwrap();
        let release_dir = temp.path().join("v0.1.0");
        std::fs::create_dir_all(&release_dir).unwrap();
        let binary_path = release_dir.join("ylnime-windows-amd64.exe");
        std::fs::write(&binary_path, b"hello").unwrap();

        let mut manifest = Manifest {
            schema: "mutsukitube-manifest-1".into(),
            version: "v0.1.0".into(),
            generated_at: "2026-10-18T00:00:00Z".into(),
            url_base: String::new(),
            artifacts: vec![Artifact {
                path: "ylnime-windows-amd64.exe".into(),
                provider: "ylnime".into(),
                os: "windows".into(),
                arch: "amd64".into(),
                sha256: sha256_hex(b"hello"),
                size: 5,
                url: String::new(),
            }],
        };

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}/v0.1.0", port);
        manifest.url_base = base_url.clone();

        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        let signature = signing_key.sign(&manifest_bytes);
        let signature_hex = hex::encode(signature.to_bytes());

        let manifest_for_server = manifest_bytes.clone();
        let signature_for_server = signature_hex.clone();
        let release_dir_for_server = release_dir.clone();

        thread::spawn(move || {
            for _ in 0..4 {
                if let Ok((mut stream, _)) = listener.accept() {
                    let mut buffer = [0u8; 1024];
                    let _ = stream.read(&mut buffer);
                    let request = String::from_utf8_lossy(&buffer);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");
                    let response_body = if path.ends_with("manifest.json") {
                        manifest_for_server.clone()
                    } else if path.ends_with("manifest.sig") {
                        signature_for_server.clone().into_bytes()
                    } else {
                        let file_name = std::path::Path::new(path).file_name().unwrap_or_default();
                        std::fs::read(release_dir_for_server.join(file_name)).unwrap_or_default()
                    };

                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        response_body.len()
                    );
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(&response_body);
                }
            }
        });

        let client = reqwest::Client::new();
        let downloaded_manifest_bytes = download_artifact(
            &client,
            &format!("{}/manifest.json", base_url),
            MAX_MANIFEST_BYTES,
        )
        .await
        .unwrap();
        let downloaded_signature =
            download_artifact(&client, &format!("{}/manifest.sig", base_url), 4096)
                .await
                .unwrap();
        let downloaded_signature_hex = String::from_utf8(downloaded_signature).unwrap();

        let verified_manifest = verify_manifest(
            &downloaded_manifest_bytes,
            &downloaded_signature_hex,
            &public_key,
        )
        .unwrap();
        let artifact = &verified_manifest.artifacts[0];
        let artifact_url = artifact_download_url(&verified_manifest, artifact).unwrap();
        let bytes = download_artifact(&client, &artifact_url, MAX_ARTIFACT_BYTES)
            .await
            .unwrap();
        verify_artifact_bytes(artifact, &bytes).unwrap();

        let install_dir = temp.path().join("installed");
        let installed_path =
            install_artifact(&install_dir, &verified_manifest.version, artifact, &bytes).unwrap();
        assert!(installed_path.exists());
        activate_version(&install_dir, &verified_manifest.version).unwrap();

        let state = load_state(&install_dir);
        assert_eq!(state.active_version.as_deref(), Some("v0.1.0"));
    }
}
