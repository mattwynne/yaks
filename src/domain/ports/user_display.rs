// Display port trait - abstraction for displaying results to user

use crate::domain::event_metadata::{Author, Timestamp};
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

    /// Display breadcrumb path (dimmed ancestor names joined with " > ", trailing " > ")
    /// No output when ancestors is empty.
    fn display_breadcrumb(&self, ancestors: &[Name]);

    /// Display context body rendered as terminal markdown
    fn display_context(&self, context: &str);

    /// Display metadata line for yx show (state, created date, author)
    fn display_metadata_line(&self, state: &str, created_at: &Timestamp, created_by: &Author);

    /// Display a log entry with event ID, author, timestamp, and message
    fn log_entry(
        &self,
        event_id: &str,
        author_name: &str,
        author_email: &str,
        timestamp: &str,
        message: &str,
    );
}
