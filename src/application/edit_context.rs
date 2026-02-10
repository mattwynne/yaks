// Use case: Edit yak context (via editor or stdin)

use crate::domain::CONTEXT_FIELD;
use anyhow::{Context as AnyhowContext, Result};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::Command;

use super::Application;

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

        // Determine how to get content based on stdin type and test mode
        let content = if !atty::is(atty::Stream::Stdin) {
            // Stdin is piped - always read from it (even in test mode)
            self.read_from_stdin()?
        } else if env::var("YX_IGNORE_STDIN").is_ok() {
            // Test mode with TTY stdin - don't open editor, return unchanged
            current_context
        } else {
            // Interactive mode (TTY) - launch editor
            self.edit_with_editor(&current_context)?
        };

        // Write updated context
        app.storage
            .write_field(&resolved_name, CONTEXT_FIELD, &content)?;
        app.log.log_command(&format!("context {}", self.name))?;

        Ok(())
    }

    fn read_from_stdin(&self) -> Result<String> {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        Ok(buffer)
    }

    fn edit_with_editor(&self, initial_content: &str) -> Result<String> {
        // Get editor from environment or default to vi
        let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

        // Create a temporary file with the current context
        let temp_file =
            tempfile::NamedTempFile::new().context("Failed to create temporary file")?;
        let temp_path = temp_file.path();

        // Write current context to temp file
        fs::write(temp_path, initial_content).context("Failed to write to temp file")?;

        // Launch editor
        let status = Command::new(&editor)
            .arg(temp_path)
            .status()
            .context(format!("Failed to launch editor: {editor}"))?;

        if !status.success() {
            anyhow::bail!("Editor exited with non-zero status");
        }

        // Read edited content
        let content = fs::read_to_string(temp_path).context("Failed to read edited content")?;

        Ok(content)
    }
}
