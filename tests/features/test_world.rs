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

    /// List all yaks
    fn list_yaks(&mut self) -> Result<()>;

    /// Get the output from the last command
    fn get_output(&self) -> String;

    /// Get the exit code from the last command
    #[allow(dead_code)]
    fn get_exit_code(&self) -> i32;
}

/// Strip ANSI color codes from output for assertions
pub fn strip_ansi_codes(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}
