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
/// - Piped stdin: Reads from stdin via `read_stdin_content`
/// - Interactive TTY: Launches $EDITOR via `edit_content`
pub struct ConsoleInput;

impl InputPort for ConsoleInput {
    fn request_content(
        &self,
        initial_content: Option<&str>,
        template: Option<&str>,
    ) -> Result<Option<String>> {
        // Try reading from stdin first (non-TTY)
        if !atty::is(atty::Stream::Stdin) {
            return self.read_stdin_content();
        }

        // YX_IGNORE_STDIN suppresses editor launch (for tests, CI,
        // non-interactive tools like Claude Code)
        if env::var("YX_IGNORE_STDIN").is_ok() {
            return Ok(None);
        }

        // Interactive mode (TTY): open editor
        self.edit_content(initial_content, template)
    }
}

impl ConsoleInput {
    /// Read content from stdin when it is piped or redirected.
    ///
    /// - If stdin is a pipe/file with content, returns
    ///   `Ok(Some(content))`
    /// - If stdin is a pipe/file with zero bytes, returns `Ok(None)`
    /// - If stdin is /dev/null or a TTY, returns `Ok(None)`
    pub fn read_stdin_content(&self) -> Result<Option<String>> {
        if Self::stdin_has_readable_data() {
            let content = Self::read_stdin()?;
            if !content.is_empty() {
                return Ok(Some(content));
            }
        }
        // Empty pipe/file or non-pipe (e.g., /dev/null in Docker):
        // treat as "no input available".
        Ok(None)
    }

    /// Open $EDITOR to let the user compose or edit content.
    ///
    /// - Pre-populates the editor with `initial` if provided,
    ///   otherwise `template`
    /// - Returns `Ok(Some(content))` if the user wrote non-empty
    ///   content that differs from the template
    /// - Returns `Ok(None)` if the content is empty or unchanged
    ///   from the template
    pub fn edit_content(
        &self,
        initial: Option<&str>,
        template: Option<&str>,
    ) -> Result<Option<String>> {
        let editor_content = initial.or(template).unwrap_or("");
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

    fn stdin_has_readable_data() -> bool {
        use std::os::unix::io::AsRawFd;

        let stdin_fd = io::stdin().as_raw_fd();

        // First check: Is it a pipe (FIFO) or a regular file
        // (redirect)?
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let stat_result = unsafe { libc::fstat(stdin_fd, &mut stat) };
        if stat_result != 0 {
            return false;
        }
        let file_type = stat.st_mode & libc::S_IFMT;
        if file_type != libc::S_IFIFO && file_type != libc::S_IFREG {
            return false; // Not a pipe or file, don't try to read
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
