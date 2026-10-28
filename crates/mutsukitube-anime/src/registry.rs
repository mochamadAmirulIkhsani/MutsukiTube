use crate::errors::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub display_name: String,
    pub executable_path: PathBuf,
    pub executable_args: Vec<String>,
    pub enabled: bool,
    pub priority: u32,
    pub expected_protocol: u32,
}

impl ProviderConfig {
    pub fn validate(&self) -> Result<(), AnimeError> {
        if self.id.trim().is_empty() {
            return Err(AnimeError::new(
                CATEGORY_INVALID_PROVIDER,
                "provider id is empty",
                false,
            ));
        }
        if self.expected_protocol != crate::models::PROTOCOL_VERSION {
            return Err(AnimeError::new(
                CATEGORY_UNSUPPORTED,
                format!(
                    "provider {} expects protocol {}, host supports {}",
                    self.id,
                    self.expected_protocol,
                    crate::models::PROTOCOL_VERSION
                ),
                false,
            ));
        }
        if !self.executable_path.is_file() {
            return Err(AnimeError::new(
                CATEGORY_INVALID_EXECUTABLE,
                format!(
                    "provider {} executable does not exist: {}",
                    self.id,
                    self.executable_path.display()
                ),
                false,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProviderRegistry {
    providers: Vec<ProviderConfig>,
}

impl ProviderRegistry {
    pub fn new(providers: Vec<ProviderConfig>) -> Self {
        let mut providers = providers;
        providers.sort_by_key(|p| p.priority);
        Self { providers }
    }

    pub fn providers(&self) -> &[ProviderConfig] {
        &self.providers
    }

    pub fn enabled_providers(&self) -> Vec<&ProviderConfig> {
        self.providers.iter().filter(|p| p.enabled).collect()
    }

    pub fn get(&self, id: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.id == id)
    }

    pub fn ids_by_priority(&self) -> Vec<String> {
        self.enabled_providers()
            .iter()
            .map(|p| p.id.clone())
            .collect()
    }

    pub fn with_standard(plugin_dir: Option<PathBuf>) -> Self {
        let ids = [
            ("otakudesu", "Otakudesu", 1),
            ("samehadaku", "Samehadaku", 2),
            ("nimegami", "Nimegami", 3),
            ("ylnime", "YLnime", 4),
            ("moenime", "Moenime", 5),
            ("astronime", "Astronime", 6),
        ];

        let mut providers = Vec::new();
        for (id, name, priority) in ids {
            let executable_path = plugin_dir
                .as_ref()
                .and_then(|dir| discover_provider_binary(dir, id))
                .unwrap_or_else(|| {
                    plugin_dir
                        .as_ref()
                        .map(|dir| dir.join(format!("{id}.exe")))
                        .unwrap_or_else(|| PathBuf::from(id))
                });

            providers.push(ProviderConfig {
                id: id.to_string(),
                display_name: name.to_string(),
                executable_path,
                executable_args: Vec::new(),
                enabled: true,
                priority,
                expected_protocol: crate::models::PROTOCOL_VERSION,
            });
        }

        Self::new(providers)
    }

    pub fn from_env() -> Self {
        Self::with_standard(plugin_dir_from_env())
    }
}

pub fn plugin_dir_from_env() -> Option<PathBuf> {
    std::env::var_os("MUTSUKITUBE_ANIME_PLUGIN_DIR").map(PathBuf::from)
}

pub fn discover_provider_binary(dir: &Path, provider_id: &str) -> Option<PathBuf> {
    for candidate in candidate_file_names(provider_id) {
        let path = dir.join(&candidate);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(entries) = fs::read_dir(dir) {
        let mut matches = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains(provider_id) {
                matches.push(path);
            }
        }
        matches.sort();
        if let Some(path) = matches.into_iter().next() {
            return Some(path);
        }
    }

    None
}

fn candidate_file_names(provider_id: &str) -> Vec<String> {
    let os_name = match std::env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "darwin",
        other => other,
    };
    let arch_name = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };

    let mut names = BTreeSet::new();
    names.insert(provider_id.to_string());
    names.insert(format!("{provider_id}.exe"));
    names.insert(format!("{provider_id}-{os_name}-{arch_name}"));
    names.insert(format!("{provider_id}-{os_name}-{arch_name}.exe"));
    names.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn discovers_exact_platform_binary() {
        let tmp = TempDir::new().unwrap();
        let name = if cfg!(windows) {
            "ylnime-windows-amd64.exe"
        } else {
            "ylnime-linux-amd64"
        };
        let path = tmp.path().join(name);
        fs::write(&path, b"").unwrap();

        let found = discover_provider_binary(tmp.path(), "ylnime").unwrap();
        assert_eq!(found, path);
    }

    #[test]
    fn invalid_executable_path_fails_validation() {
        let config = ProviderConfig {
            id: "ylnime".into(),
            display_name: "YLnime".into(),
            executable_path: PathBuf::from("definitely-missing.exe"),
            executable_args: Vec::new(),
            enabled: true,
            priority: 1,
            expected_protocol: crate::models::PROTOCOL_VERSION,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn disabled_provider_is_filtered() {
        let config = ProviderConfig {
            id: "ylnime".into(),
            display_name: "YLnime".into(),
            executable_path: PathBuf::from("x"),
            executable_args: Vec::new(),
            enabled: false,
            priority: 1,
            expected_protocol: crate::models::PROTOCOL_VERSION,
        };
        let registry = ProviderRegistry::new(vec![config]);
        assert!(registry.enabled_providers().is_empty());
    }

    #[test]
    fn priority_ordering_is_stable() {
        let a = ProviderConfig {
            id: "a".into(),
            display_name: "A".into(),
            executable_path: PathBuf::from("a"),
            executable_args: Vec::new(),
            enabled: true,
            priority: 2,
            expected_protocol: crate::models::PROTOCOL_VERSION,
        };
        let b = ProviderConfig {
            id: "b".into(),
            display_name: "B".into(),
            executable_path: PathBuf::from("b"),
            executable_args: Vec::new(),
            enabled: true,
            priority: 1,
            expected_protocol: crate::models::PROTOCOL_VERSION,
        };
        let registry = ProviderRegistry::new(vec![a, b]);
        assert_eq!(registry.ids_by_priority(), vec!["b", "a"]);
    }

    #[test]
    fn protocol_incompatibility_fails_validation() {
        let config = ProviderConfig {
            id: "ylnime".into(),
            display_name: "YLnime".into(),
            executable_path: PathBuf::from("x"),
            executable_args: Vec::new(),
            enabled: true,
            priority: 1,
            expected_protocol: 999,
        };
        assert!(config.validate().is_err());
    }
}
