// Display port trait - abstraction for displaying results to user

use crate::adapters::views::{LogEntryView, Message, YakDetailView, YakTreeView};

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

    /// Display help text to the user.
    fn show_help(&self, help_text: &str);

    /// Start a progress spinner with the given message.
    /// Returns a handle that stops the spinner when dropped.
    /// Default impl returns a no-op handle.
    fn start_progress(&self, _message: &str) -> Box<dyn ProgressHandle> {
        Box::new(NoOpProgressHandle)
    }
}

/// Handle to a running progress indicator. Stops on drop.
pub trait ProgressHandle {}

/// No-op progress handle for adapters that don't support spinners.
struct NoOpProgressHandle;
impl ProgressHandle for NoOpProgressHandle {}
impl Drop for NoOpProgressHandle {
    fn drop(&mut self) {}
}
