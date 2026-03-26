// User configuration port - abstraction for reading/writing user preferences

use anyhow::Result;

/// Known config keys with their default values
pub const CONFIG_KEYS: &[(&str, &str)] = &[("show-claude-plugin-hint", "true")];

/// Port for user-level configuration (not event-sourced).
///
/// Config is simple key-value storage for user preferences.
/// Each key has a known default value defined in CONFIG_KEYS.
pub trait UserConfigPort {
    /// Get a config value by key. Returns the default if unset.
    fn get(&self, key: &str) -> Result<String>;

    /// Set a config value.
    fn set(&mut self, key: &str, value: &str) -> Result<()>;

    /// List all config keys with their current values (or defaults).
    fn list(&self) -> Result<Vec<(String, String)>>;
}

/// Look up the default value for a config key.
pub fn default_for_key(key: &str) -> Option<&'static str> {
    CONFIG_KEYS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}
