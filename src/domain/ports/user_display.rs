// Display port trait - abstraction for displaying results to user

use crate::domain::slug::Name;

pub trait DisplayPort {
    /// Display success message
    fn success(&self, message: &str);

    /// Display informational message
    fn info(&self, message: &str);

    /// Display a yak entry in pretty format (tree-drawing with colored status)
    fn display_yak_pretty(&self, prefix: &str, name: &Name, state: &str);

    /// Display a yak entry in markdown format
    fn display_yak_markdown(&self, depth: usize, name: &Name, state: &str);
}
