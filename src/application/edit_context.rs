// Use case: Edit yak context (via editor or stdin)

use crate::domain::CONTEXT_FIELD;
use anyhow::Result;

use super::{Application, UseCase};

pub struct EditContext {
    name: String,
}

impl EditContext {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &Application) -> Result<()> {
        // Resolve yak name (exact or fuzzy match)
        let resolved_name = app.storage.find_yak(&self.name)?;

        // Read current context
        let current_context = app
            .storage
            .read_field(&resolved_name, CONTEXT_FIELD)
            .unwrap_or_default();

        // Request edited content via input port
        let content =
            if let Some(edited) = app.input.request_content(Some(&current_context), None)? {
                edited
            } else {
                // No input provided, keep current content
                current_context
            };

        // Write updated context
        app.storage
            .write_field(&resolved_name, CONTEXT_FIELD, &content)?;
        app.log.log_command(&format!("context {}", self.name))?;

        Ok(())
    }
}

impl UseCase for EditContext {
    fn execute(&self, app: &Application) -> Result<()> {
        Self::execute(self, app)
    }
}
