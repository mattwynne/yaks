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
        // Always read piped stdin, even in YX_IGNORE_STDIN mode.
        // A pipe means the user explicitly provided content.
        if !atty::is(atty::Stream::Stdin) {
            if Self::stdin_is_pipe_or_file() {
                let content = Self::read_stdin()?;
                if !content.is_empty() {
                    return Ok(Some(content));
                }
                return Err(anyhow::anyhow!("no content received on stdin"));
            }
            return Ok(None);
        }

        // YX_IGNORE_STDIN suppresses editor launch (for tests, CI,
        // non-interactive tools like Claude Code)
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
    fn stdin_is_pipe_or_file() -> bool {
        use std::os::unix::io::AsRawFd;

        let stdin_fd = io::stdin().as_raw_fd();

        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let stat_result = unsafe { libc::fstat(stdin_fd, &mut stat) };
        if stat_result != 0 {
            return false;
        }
        let file_type = stat.st_mode & libc::S_IFMT;
        file_type == libc::S_IFIFO || file_type == libc::S_IFREG
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
