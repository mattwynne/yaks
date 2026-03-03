// JsonDisplay adapter - serializes view models to JSON on stdout

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::narrative::NarrativeSpan;
use crate::domain::ports::DisplayPort;
use crate::domain::slug::Name;
use crate::domain::views::{LogEntryView, Message, YakDetailView, YakTreeView};

pub struct JsonDisplay;

impl Default for JsonDisplay {
    fn default() -> Self {
        Self
    }
}

impl JsonDisplay {
    pub fn new() -> Self {
        Self
    }
}

impl DisplayPort for JsonDisplay {
    fn width(&self) -> usize {
        0 // Not used for JSON output
    }

    fn show_yak(&self, view: &YakDetailView) {
        let json = serde_json::to_string_pretty(view).expect("Failed to serialize YakDetailView");
        println!("{json}");
    }

    fn show_list(&self, view: &YakTreeView) {
        let json =
            serde_json::to_string_pretty(&view.nodes).expect("Failed to serialize YakTreeView");
        println!("{json}");
    }

    fn show_log(&self, entries: &[LogEntryView]) {
        let json = serde_json::to_string_pretty(entries).expect("Failed to serialize log entries");
        println!("{json}");
    }

    fn message(&self, msg: &Message) {
        match msg {
            Message::Warn(s) => {
                eprintln!("Warning: {s}");
            }
            Message::Info(s) | Message::Success(s) => {
                println!("{s}");
            }
            Message::Hint(s) => {
                // Hints are typically not shown in JSON mode, but print if present
                println!("{s}");
            }
        }
    }

    // Old methods - JsonDisplay is new and only uses the high-level methods
    fn display_hint(&self, _message: &str) {
        unimplemented!("Use message() instead")
    }

    fn success(&self, _message: &str) {
        unimplemented!("Use message() instead")
    }

    fn info(&self, _message: &str) {
        unimplemented!("Use message() instead")
    }

    fn warn(&self, _message: &str) {
        unimplemented!("Use message() instead")
    }

    fn display_yak_pretty(&self, _prefix: &str, _name: &Name, _state: &str, _tags: &[String]) {
        unimplemented!("Use show_list() instead")
    }

    fn display_yak_markdown(&self, _depth: usize, _name: &Name, _state: &str, _tags: &[String]) {
        unimplemented!("Use show_list() instead")
    }

    fn display_header_box(
        &self,
        _ancestors: &[Name],
        _name: &Name,
        _state: &str,
        _created_at: &Timestamp,
        _created_by: &Author,
        _children: &[(Name, String)],
        _fields: &[(String, String)],
        _tags: &[String],
    ) {
        unimplemented!("Use show_yak() instead")
    }

    fn display_breadcrumb(&self, _ancestors: &[Name]) {
        unimplemented!("Use show_yak() instead")
    }

    fn display_section_rule(&self, _label: &str) {
        unimplemented!("Use show_yak() instead")
    }

    fn display_closing_rule(&self) {
        unimplemented!("Use show_yak() instead")
    }

    fn display_context(&self, _context: &str) {
        unimplemented!("Use show_yak() instead")
    }

    fn display_metadata_line(&self, _state: &str, _created_at: &Timestamp, _created_by: &Author) {
        unimplemented!("Use show_yak() instead")
    }

    fn log_entry(
        &self,
        _narrative: &[NarrativeSpan],
        _timestamp: &str,
        _event_id: &str,
        _commit_sha: Option<&str>,
    ) {
        unimplemented!("Use show_log() instead")
    }
}
