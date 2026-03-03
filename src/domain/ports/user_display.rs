// Display port trait - abstraction for displaying results to user

use crate::domain::views::{LogEntryView, Message, YakDetailView, YakTreeView};

pub trait DisplayPort {
    /// Get the display width
    fn width(&self) -> usize;

    /// Display a complete yak detail view (for yx show)
    fn show_yak(&self, view: &YakDetailView);

    /// Display a complete yak tree/list view (for yx ls)
    fn show_list(&self, view: &YakTreeView);

    /// Display log entries (for yx log)
    fn show_log(&self, entries: &[LogEntryView]);

    /// Display a user-facing message (hint, success, info, warning)
    fn message(&self, msg: &Message);

    /// Display a progress indicator (spinner with message).
    /// Called repeatedly during long operations.
    /// Default impl does nothing (non-TUI adapters don't need spinners).
    fn progress(&self, _message: &str) {}
}
