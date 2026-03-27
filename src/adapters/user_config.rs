// User config adapters - TOML file-based and in-memory implementations

use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::domain::ports::user_config::{default_for_key, UserConfigPort, CONFIG_KEYS};

/// TOML file-based user config adapter.
///
/// Reads/writes config from $XDG_CONFIG_HOME/yaks/config.toml
/// (defaults to ~/.config/yaks/config.toml).
pub struct TomlFileConfig {
    path: PathBuf,
    values: HashMap<String, String>,
}

impl TomlFileConfig {
    pub fn new() -> Result<Self> {
        let path = config_file_path()?;
        let values = if path.exists() {
            read_toml_file(&path)?
        } else {
            HashMap::new()
        };
        Ok(Self { path, values })
    }

    /// Create with an explicit path (for testing).
    pub fn with_path(path: PathBuf) -> Result<Self> {
        let values = if path.exists() {
            read_toml_file(&path)?
        } else {
            HashMap::new()
        };
        Ok(Self { path, values })
    }

    fn write(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut content = String::new();
        let mut keys: Vec<_> = self.values.keys().collect();
        keys.sort();
        for key in keys {
            content.push_str(&format!("{} = \"{}\"\n", key, self.values[key]));
        }
        std::fs::write(&self.path, content)?;
        Ok(())
    }
}

impl UserConfigPort for TomlFileConfig {
    fn get(&self, key: &str) -> Result<String> {
        if let Some(value) = self.values.get(key) {
            Ok(value.clone())
        } else if let Some(default) = default_for_key(key) {
            Ok(default.to_string())
        } else {
            bail!("Unknown config key: {}", key)
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        if default_for_key(key).is_none() {
            bail!("Unknown config key: {}", key);
        }
        self.values.insert(key.to_string(), value.to_string());
        self.write()
    }

    fn list(&self) -> Result<Vec<(String, String)>> {
        let mut result = Vec::new();
        for (key, default) in CONFIG_KEYS {
            let value = self.values.get(*key).map(|s| s.as_str()).unwrap_or(default);
            result.push((key.to_string(), value.to_string()));
        }
        Ok(result)
    }
}

/// In-memory config adapter for testing.
#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
pub struct InMemoryConfig {
    values: HashMap<String, String>,
}

#[cfg(any(test, feature = "test-support"))]
impl InMemoryConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(any(test, feature = "test-support"))]
impl UserConfigPort for InMemoryConfig {
    fn get(&self, key: &str) -> Result<String> {
        if let Some(value) = self.values.get(key) {
            Ok(value.clone())
        } else if let Some(default) = default_for_key(key) {
            Ok(default.to_string())
        } else {
            bail!("Unknown config key: {}", key)
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        if default_for_key(key).is_none() {
            bail!("Unknown config key: {}", key);
        }
        self.values.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn list(&self) -> Result<Vec<(String, String)>> {
        let mut result = Vec::new();
        for (key, default) in CONFIG_KEYS {
            let value = self.values.get(*key).map(|s| s.as_str()).unwrap_or(default);
            result.push((key.to_string(), value.to_string()));
        }
        Ok(result)
    }
}

/// Resolve the config file path.
fn config_file_path() -> Result<PathBuf> {
    let config_dir = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if let Some(home) = dirs_home() {
        home.join(".config")
    } else {
        bail!("Could not determine home directory");
    };
    Ok(config_dir.join("yaks").join("config.toml"))
}

/// Read a simple TOML key-value file.
fn read_toml_file(path: &PathBuf) -> Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().trim_matches('"').to_string();
            map.insert(key, value);
        }
    }
    Ok(map)
}

/// Get the user's home directory.
fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_toml_file_skips_comments_and_empty_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "# This is a comment\n\nshow-claude-plugin-hint = \"false\"\n",
        )
        .unwrap();
        let map = read_toml_file(&path).unwrap();
        assert_eq!(map.get("show-claude-plugin-hint").unwrap(), "false");
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn read_toml_file_skips_comments_containing_equals() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "# default = \"something\"\nshow-claude-plugin-hint = \"false\"\n",
        )
        .unwrap();
        let map = read_toml_file(&path).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("show-claude-plugin-hint").unwrap(), "false");
        assert!(map.get("# default").is_none());
    }

    #[test]
    fn read_toml_file_handles_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "").unwrap();
        let map = read_toml_file(&path).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn dirs_home_returns_home_env_var() {
        // HOME is always set in test environments
        let result = dirs_home();
        assert!(result.is_some());
        let home = result.unwrap();
        assert!(!home.as_os_str().is_empty());
        assert_eq!(home, PathBuf::from(std::env::var("HOME").unwrap()));
    }

    #[test]
    fn list_returns_all_config_keys_with_defaults() {
        let config = TomlFileConfig {
            path: PathBuf::from("/nonexistent"),
            values: HashMap::new(),
        };
        let entries = config.list().unwrap();
        assert_eq!(entries.len(), CONFIG_KEYS.len());
        assert!(entries.len() > 0);
        for (i, (key, default)) in CONFIG_KEYS.iter().enumerate() {
            assert_eq!(entries[i].0, *key);
            assert_eq!(entries[i].1, *default);
        }
    }

    #[test]
    fn list_returns_overridden_values() {
        let mut values = HashMap::new();
        values.insert("show-claude-plugin-hint".to_string(), "false".to_string());
        let config = TomlFileConfig {
            path: PathBuf::from("/nonexistent"),
            values,
        };
        let entries = config.list().unwrap();
        assert_eq!(entries.len(), CONFIG_KEYS.len());
        assert_eq!(entries[0].0, "show-claude-plugin-hint");
        assert_eq!(entries[0].1, "false");
    }
}
