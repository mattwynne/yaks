// Display port trait - abstraction for displaying results to user

pub trait DisplayPort {
    /// Display success message
    fn success(&self, message: &str);

    /// Display error message
    #[allow(dead_code)] // Part of port API, used by test adapters
    fn error(&self, message: &str);

    /// Display informational message
    fn info(&self, message: &str);

    /// Display a yak entry in pretty format (tree-drawing with colored status)
    fn display_yak_pretty(&self, prefix: &str, name: &str, state: &str);

    /// Display a yak entry in markdown format
    fn display_yak_markdown(&self, depth: usize, name: &str, state: &str);
}
