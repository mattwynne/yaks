// Use case: Add a new yak

use crate::domain::{validate_yak_name, CONTEXT_FIELD};
use anyhow::{Context as AnyhowContext, Result};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::Command;

use super::Application;

/// AddYak use case - creates a new yak
pub struct AddYak {
    name: String,
}

impl AddYak {
    /// Create a new AddYak use case with the yak name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Execute the use case with the application's infrastructure
    pub fn execute(&self, app: &Application) -> Result<()> {
        // Validate yak name
        validate_yak_name(&self.name).map_err(|e| anyhow::anyhow!(e))?;

        app.storage.create_yak(&self.name)?;

        // In test mode, skip all interactive behavior (editor launch, stdin reading)
        if env::var("YX_IGNORE_STDIN").is_ok() {
            // Test mode: just create empty yak
        } else if !atty::is(atty::Stream::Stdin) {
            // Non-TTY: Check if stdin has context piped to it
            if Self::stdin_has_readable_data() {
                // Read context from stdin
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                if !buffer.is_empty() {
                    app.storage
                        .write_field(&self.name, CONTEXT_FIELD, &buffer)?;
                }
            }
            // If no readable data, just create empty yak
        } else {
            // Interactive mode (TTY): open editor with template
            let template = self.generate_context_template()?;
            let edited_content = self.edit_with_editor(&template)?;

            // Only save if there's actual content (not just the template)
            if !edited_content.trim().is_empty() && edited_content.trim() != template.trim() {
                app.storage
                    .write_field(&self.name, CONTEXT_FIELD, &edited_content)?;
            }
        }

        app.log.log_command(&format!("add {}", self.name))?;
        Ok(())
    }

    fn stdin_has_readable_data() -> bool {
        // Defense in depth: double-check YX_IGNORE_STDIN even though execute() already checks it
        // This provides safety if this method is called from other code paths in the future
        if env::var("YX_IGNORE_STDIN").is_ok() {
            return false;
        }

        use std::os::unix::io::AsRawFd;

        let stdin_fd = io::stdin().as_raw_fd();

        // First check: Is it actually a pipe (FIFO)?
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let stat_result = unsafe { libc::fstat(stdin_fd, &mut stat) };
        if stat_result != 0 || (stat.st_mode & libc::S_IFMT) != libc::S_IFIFO {
            return false; // Not a pipe, don't try to read
        }

        // Second check: Is there data available to read?
        let mut pollfd = libc::pollfd {
            fd: stdin_fd,
            events: libc::POLLIN,
            revents: 0,
        };

        // Poll with 0 timeout (non-blocking check)
        let result = unsafe { libc::poll(&mut pollfd, 1, 0) };

        // Return true only if:
        // 1. It's a pipe (checked above)
        // 2. Poll succeeded
        // 3. POLLIN is set (data available)
        result > 0 && (pollfd.revents & libc::POLLIN) != 0
    }

    fn generate_context_template(&self) -> Result<String> {
        // Parse the yak hierarchy (e.g., "make tea/add milk/go to shops")
        let parts: Vec<&str> = self.name.split('/').collect();

        if parts.len() == 1 {
            // Simple yak, no parents
            return Ok(format!("# {}\n\n", self.name));
        }

        // Nested yak - generate template with parent chain
        let leaf = parts.last().unwrap();
        let mut template = format!("# {}\n\nWhy?\n\n", leaf);

        // Build the parent chain explanation
        for i in 0..parts.len() - 1 {
            let parent_path = parts[0..=i].join("/");
            let parent_name = parts[i];

            if i == 0 {
                template.push_str(&format!(
                    "* We want to *{}* (see `yx context \"{}\"`)\n",
                    parent_name, parent_path
                ));
            } else {
                let prev_parent = parts[i - 1];
                template.push_str(&format!(
                    "* to {}, we need to *{}* (see `yx context \"{}\"`)\n",
                    prev_parent, parent_name, parent_path
                ));
            }
        }

        // Add the final item explaining the current yak
        let last_parent = parts[parts.len() - 2];
        template.push_str(&format!("* to {}, we need to *{}*\n", last_parent, leaf));

        Ok(template)
    }

    fn edit_with_editor(&self, initial_content: &str) -> Result<String> {
        // Get editor from environment or default to vi
        let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

        // Create a temporary file with the template
        let temp_file =
            tempfile::NamedTempFile::new().context("Failed to create temporary file")?;
        let temp_path = temp_file.path();

        // Write template to temp file
        fs::write(temp_path, initial_content).context("Failed to write template to temp file")?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{InMemoryDisplay, InMemoryLog, InMemoryStorage};
    use crate::ports::StoragePort;

    #[test]
    fn test_add_yak_creates_yak() {
        // Prevent editor from opening in test environment
        env::set_var("YX_IGNORE_STDIN", "1");

        let storage = InMemoryStorage::new();
        let display = InMemoryDisplay::new();
        let log = InMemoryLog::new();
        let app = Application::new(&storage, &display, &log);

        let use_case = AddYak::new("test-yak");
        use_case.execute(&app).unwrap();

        assert!(storage.get_yak("test-yak").is_ok());
    }

    #[test]
    fn test_generate_context_template_simple_yak() {
        let use_case = AddYak::new("simple-yak");
        let template = use_case.generate_context_template().unwrap();
        assert_eq!(template, "# simple-yak\n\n");
    }

    #[test]
    fn test_generate_context_template_nested_yak() {
        let use_case = AddYak::new("make tea/add milk/go to shops");
        let template = use_case.generate_context_template().unwrap();

        let expected = "# go to shops\n\nWhy?\n\n\
            * We want to *make tea* (see `yx context \"make tea\"`)\n\
            * to make tea, we need to *add milk* (see `yx context \"make tea/add milk\"`)\n\
            * to add milk, we need to *go to shops*\n";

        assert_eq!(template, expected);
    }
}
