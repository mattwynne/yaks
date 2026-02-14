// Trait defining operations for testing yak commands
// Implemented by both FullStackWorld and InProcessWorld

use anyhow::Result;

/// TestWorld defines the operations available in Cucumber tests
///
/// Two implementations:
/// - FullStackWorld: spawns yx binary (real integration test)
/// - InProcessWorld: calls CommandHandler directly (fast unit-like test)
pub trait TestWorld {
    /// Add a yak with the given name
    fn add_yak(&mut self, name: &str) -> Result<()>;

    /// Mark a yak as done
    fn done_yak(&mut self, name: &str) -> Result<()>;

    /// List all yaks (default pretty format)
    fn list_yaks(&mut self) -> Result<()>;

    /// List yaks with a specific format
    fn list_yaks_with_format(&mut self, format: &str) -> Result<()>;

    /// List yaks with a specific format and filter
    fn list_yaks_with_format_and_filter(&mut self, format: &str, only: &str) -> Result<()>;

    /// Get the output from the last command
    fn get_output(&self) -> String;

    /// Try to add a yak - captures result without bailing on failure
    fn try_add_yak(&mut self, name: &str) -> Result<()>;

    /// Remove a yak by name
    fn remove_yak(&mut self, name: &str) -> Result<()>;

    /// Try to remove a yak - captures result without bailing on failure
    fn try_remove_yak(&mut self, name: &str) -> Result<()>;

    /// Get the error output from the last command
    fn get_error(&self) -> String;

    /// Set a yak's context from content (simulates stdin piping)
    fn set_context(&mut self, name: &str, content: &str) -> Result<()>;

    /// Show a yak's context (yx context --show)
    fn show_context(&mut self, name: &str) -> Result<()>;

    /// Try to mark a yak as done - captures result without bailing on failure
    fn try_done_yak(&mut self, name: &str) -> Result<()>;

    /// Mark a yak as done recursively (parent + all descendants)
    fn done_yak_recursive(&mut self, name: &str) -> Result<()>;

    /// Prune all done yaks
    fn prune_yaks(&mut self) -> Result<()>;

    /// Set a yak's state (todo, wip, done)
    fn set_state(&mut self, name: &str, state: &str) -> Result<()>;

    /// Try to set a yak's state - captures result without bailing on failure
    fn try_set_state(&mut self, name: &str, state: &str) -> Result<()>;

    /// Start a yak (alias for setting state to wip)
    fn start_yak(&mut self, name: &str) -> Result<()>;

    /// Move/rename a yak
    fn move_yak(&mut self, from: &str, to: &str) -> Result<()>;

    /// Set a yak's field from content (simulates stdin piping)
    fn set_field(&mut self, name: &str, field: &str, content: &str) -> Result<()>;

    /// Try to set a yak's field - captures result without bailing on failure
    fn try_set_field(&mut self, name: &str, field: &str, content: &str) -> Result<()>;

    /// Show a yak's field (yx field --show)
    fn show_field(&mut self, name: &str, field: &str) -> Result<()>;

    /// Get the exit code from the last command
    fn get_exit_code(&self) -> i32;
}

/// Strip ANSI color codes from output for assertions
pub fn strip_ansi_codes(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}
