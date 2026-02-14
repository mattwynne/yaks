// Console input adapter - handles stdin and editor-based input

use crate::domain::ports::InputPort;
use anyhow::{Context as AnyhowContext, Result};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::Command;

/// Console-based input adapter
///
/// Handles different input modes:
/// - Test mode: Returns None (via YX_IGNORE_STDIN)
/// - Piped stdin: Reads from stdin
/// - Interactive TTY: Launches $EDITOR
pub struct ConsoleInput;

impl InputPort for ConsoleInput {
    fn request_content(
        &self,
        initial_content: Option<&str>,
        template: Option<&str>,
    ) -> Result<Option<String>> {
        // Check if stdin is a TTY
        if !atty::is(atty::Stream::Stdin) {
            // Non-TTY: Check if stdin has content piped to it
            if Self::stdin_has_readable_data() {
                let content = Self::read_stdin()?;
                if !content.is_empty() {
                    return Ok(Some(content));
                }
            }
            // No readable data, return None
            return Ok(None);
        }

        // TTY: check if we should skip interactive behavior (test mode)
        if env::var("YX_IGNORE_STDIN").is_ok() {
            return Ok(None);
        }

        // Interactive mode (TTY): open editor
        let editor_content = initial_content.or(template).unwrap_or("");
        let edited = Self::edit_with_editor(editor_content)?;

        // Only return content if it differs from template
        if !edited.trim().is_empty()
            && (template.is_none() || edited.trim() != template.unwrap().trim())
        {
            Ok(Some(edited))
        } else {
            Ok(None)
        }
    }
}

impl ConsoleInput {
    fn stdin_has_readable_data() -> bool {
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

        // Return true only if poll succeeded and POLLIN is set
        result > 0 && (pollfd.revents & libc::POLLIN) != 0
    }

    fn read_stdin() -> Result<String> {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        Ok(buffer)
    }

    fn edit_with_editor(initial_content: &str) -> Result<String> {
        // Get editor from environment or default to vi
        let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

        // Create a temporary file with the initial content
        let temp_file =
            tempfile::NamedTempFile::new().context("Failed to create temporary file")?;
        let temp_path = temp_file.path();

        // Write initial content to temp file
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
