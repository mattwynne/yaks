// Use case: Get a user config value

use crate::adapters::views::Message;
use anyhow::Result;

use super::{Application, UseCase};

pub struct GetConfig {
    key: String,
}

impl GetConfig {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
        }
    }
}

impl UseCase for GetConfig {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let config = app
            .user_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("User config not available"))?;
        let value = config.get(&self.key)?;
        app.display.message(&Message::Info(value));
        Ok(())
    }
}
