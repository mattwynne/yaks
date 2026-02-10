// Use case: Show yak context

use crate::domain::CONTEXT_FIELD;
use anyhow::Result;

use super::Application;

pub struct ShowContext {
    name: String,
}

impl ShowContext {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &Application) -> Result<()> {
        // Resolve yak name (exact or fuzzy match)
        let resolved_name = app.storage.find_yak(&self.name)?;

        // Read context
        let context = app
            .storage
            .read_field(&resolved_name, CONTEXT_FIELD)
            .unwrap_or_default();

        // Display the header (yak name)
        app.display.info(&resolved_name);

        // Display a blank line if there's content
        if !context.is_empty() {
            app.display.info("");
            // Display the context
            app.display.info(&context);
        }

        Ok(())
    }
}
