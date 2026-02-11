// Log port - commits yak operations to git history

use anyhow::Result;

#[allow(dead_code)]
pub trait LogPort {
    /// Log a command by committing current .yaks state to refs/notes/yaks
    fn log_command(&self, command: &str) -> Result<()>;
}
