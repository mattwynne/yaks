// Use case: List all user config values

use crate::adapters::views::Message;
use anyhow::Result;

use super::{Application, UseCase};

#[derive(Default)]
pub struct ListConfig;

impl ListConfig {
    pub fn new() -> Self {
        Self
    }
}

impl UseCase for ListConfig {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let config = app
            .user_config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("User config not available"))?;
        let entries = config.list()?;
        let lines: Vec<String> = entries
            .iter()
            .map(|(k, v)| format!("{} = {}", k, v))
            .collect();
        app.display.message(&Message::Info(lines.join("\n")));
        Ok(())
    }
}
