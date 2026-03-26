// Use case: Set a user config value

use anyhow::Result;

use super::{Application, UseCase};

pub struct SetConfig {
    key: String,
    value: String,
}

impl SetConfig {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

impl UseCase for SetConfig {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let config = app
            .user_config
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("User config not available"))?;
        config.set(&self.key, &self.value)?;
        Ok(())
    }
}
