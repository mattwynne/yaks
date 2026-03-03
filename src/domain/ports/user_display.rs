// Display port trait - abstraction for displaying results to user

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::narrative::NarrativeSpan;
use crate::domain::slug::Name;
use crate::domain::views::{LogEntryView, Message, YakDetailView, YakTreeView};

pub trait DisplayPort {
    /// Get the display width
    fn width(&self) -> usize;

    /// Display a hint message (grey italic)
    fn display_hint(&self, message: &str);

    /// Display success message
    fn success(&self, message: &str);

    /// Display informational message
    fn info(&self, message: &str);

    /// Display warning message (to stderr)
    fn warn(&self, message: &str);

    /// Display a yak entry in pretty format (tree-drawing with colored status)
    fn display_yak_pretty(&self, prefix: &str, name: &Name, state: &str, tags: &[String]);

    /// Display a yak entry in markdown format
    fn display_yak_markdown(&self, depth: usize, name: &Name, state: &str, tags: &[String]);

    /// Display header box with breadcrumb, name, state, date, author, children, and short fields
    #[allow(clippy::too_many_arguments)]
    fn display_header_box(
        &self,
        ancestors: &[Name],
        name: &Name,
        state: &str,
        created_at: &Timestamp,
        created_by: &Author,
        children: &[(Name, String)],
        fields: &[(String, String)],
        tags: &[String],
    );

    /// Display breadcrumb path (dimmed ancestor names joined with " > ", trailing " > ")
    /// No output when ancestors is empty.
    fn display_breadcrumb(&self, ancestors: &[Name]);

    /// Display a section rule with a label: ── label ────────
    fn display_section_rule(&self, label: &str);

    /// Display a closing rule: ────────────────────────
    fn display_closing_rule(&self);

    /// Display context body rendered as terminal markdown
    fn display_context(&self, context: &str);

    /// Display metadata line for yx show (state, created date, author)
    fn display_metadata_line(&self, state: &str, created_at: &Timestamp, created_by: &Author);

    /// Display a log entry with narrative spans, timestamp, event ID, and optional commit SHA
    fn log_entry(
        &self,
        narrative: &[NarrativeSpan],
        timestamp: &str,
        event_id: &str,
        commit_sha: Option<&str>,
    );

    /// Display a complete yak detail view (for yx show)
    fn show_yak(&self, _view: &YakDetailView) {
        unimplemented!("DisplayPort::show_yak not yet implemented for this adapter")
    }

    /// Display a complete yak tree/list view (for yx ls)
    fn show_list(&self, _view: &YakTreeView) {
        unimplemented!("DisplayPort::show_list not yet implemented for this adapter")
    }

    /// Display log entries (for yx log)
    fn show_log(&self, _entries: &[LogEntryView]) {
        unimplemented!("DisplayPort::show_log not yet implemented for this adapter")
    }

    /// Display a user-facing message (hint, success, info, warning)
    fn message(&self, _msg: &Message) {
        unimplemented!("DisplayPort::message not yet implemented for this adapter")
    }
}
