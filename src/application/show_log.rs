// ShowLog use case - displays the event log

use anyhow::Result;
use chrono::DateTime;

use super::{Application, UseCase};
use crate::domain::YakEvent;

pub struct ShowLog;

impl Default for ShowLog {
    fn default() -> Self {
        Self
    }
}

impl ShowLog {
    pub fn new() -> Self {
        Self
    }
}

impl UseCase for ShowLog {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let reader = app
            .event_reader
            .ok_or_else(|| anyhow::anyhow!("Event reader not configured"))?;
        let events = reader.get_all_events()?;

        for (entry_count, event) in events.iter().enumerate() {
            if entry_count > 0 {
                app.display.info("");
            }

            let meta = event.metadata();
            let event_id = meta.event_id.as_deref().unwrap_or("-");
            let datetime =
                DateTime::from_timestamp(meta.timestamp.as_epoch_secs(), 0).unwrap_or_default();
            let formatted_time = datetime.format("%Y-%m-%d %H:%M").to_string();

            app.display.log_entry(
                event_id,
                &meta.author.name,
                &meta.author.email,
                &formatted_time,
                &event.format_message(),
            );

            if let YakEvent::Compacted(snapshots, _) = event {
                for snap in snapshots {
                    app.display
                        .info(&format!("        Added: \"{}\" \"{}\"", snap.name, snap.id));
                    if snap.state != "todo" {
                        app.display
                            .info(&format!("        FieldUpdated: \"{}\" \"state\"", snap.id));
                    }
                    if let Some(context) = &snap.context {
                        if !context.is_empty() {
                            app.display.info(&format!(
                                "        FieldUpdated: \"{}\" \"context.md\"",
                                snap.id
                            ));
                        }
                    }
                    let mut field_names: Vec<&String> = snap.fields.keys().collect();
                    field_names.sort();
                    for field_name in field_names {
                        app.display.info(&format!(
                            "        FieldUpdated: \"{}\" \"{}\"",
                            snap.id, field_name
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::{AddYak, CompactEvents};
    use crate::infrastructure::EventBus;

    #[test]
    fn test_show_log_displays_events() {
        let mut event_store = InMemoryEventStore::new();
        let reader = event_store.clone();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();

        let auth = InMemoryAuthentication::new();
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            Some(&reader),
            &auth,
        );

        app.handle(AddYak::new("test yak")).unwrap();
        buffer.clear();
        app.handle(ShowLog::new()).unwrap();

        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        assert!(
            lines[0].starts_with("event "),
            "Expected first line to start with 'event ', got: {:?}",
            lines[0]
        );
        assert!(
            lines.iter().any(|m| m.contains("test@test.com")),
            "Expected log to contain author email 'test@test.com', got: {:?}",
            lines
        );
        assert!(
            lines.iter().any(|m| m.contains("Added")),
            "Expected log to contain 'Added' event message, got: {:?}",
            lines
        );
    }

    #[test]
    fn test_show_log_fails_when_not_configured() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();

        let auth = InMemoryAuthentication::new();
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            &auth,
        );

        let result = app.handle(ShowLog::new());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Event reader not configured"
        );
    }

    #[test]
    fn test_show_log_uses_git_log_style_format() {
        let mut event_store = InMemoryEventStore::new();
        let reader = event_store.clone();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();

        let auth = InMemoryAuthentication::new();
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            Some(&reader),
            &auth,
        );

        app.handle(AddYak::new("first yak")).unwrap();
        app.handle(AddYak::new("second yak")).unwrap();

        buffer.clear();

        app.handle(ShowLog::new()).unwrap();

        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        // Each event is 5 lines: event, Author, Date, blank, message
        // Between events there's a blank separator line
        // So 2 events = 5 + 1 + 5 = 11 lines
        assert_eq!(
            lines.len(),
            11,
            "Expected 11 lines for 2 events, got {}. Lines: {:?}",
            lines.len(),
            lines
        );
        assert!(lines[0].starts_with("event "), "Line 1: {:?}", lines[0]);
        assert!(lines[1].starts_with("Author: "), "Line 2: {:?}", lines[1]);
        assert!(lines[2].starts_with("Date:   "), "Line 3: {:?}", lines[2]);
        assert!(lines[3].is_empty(), "Line 4 should be blank");
        assert!(
            lines[4].starts_with("    "),
            "Line 5 should be indented: {:?}",
            lines[4]
        );
        assert!(lines[5].is_empty(), "Line 6 should be separator");
        assert!(lines[6].starts_with("event "), "Line 7: {:?}", lines[6]);
    }

    #[test]
    fn test_show_log_compacted_shows_all_snapshot_fields() {
        let mut event_store = InMemoryEventStore::new();
        let reader = event_store.clone();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();

        let auth = InMemoryAuthentication::new();
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            Some(&reader),
            &auth,
        );

        app.handle(
            AddYak::new("test yak")
                .with_context(Some("some context notes"))
                .with_state(Some("wip"))
                .with_field("plan", "step 1"),
        )
        .unwrap();
        app.handle(CompactEvents::new()).unwrap();

        buffer.clear();
        app.handle(ShowLog::new()).unwrap();

        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        // Find the Compacted section and check its sub-lines
        let compacted_idx = lines
            .iter()
            .position(|l| l.contains("Compacted"))
            .expect("Expected a Compacted line in output");
        let compacted_lines: Vec<&&str> = lines[compacted_idx..].iter().collect();

        assert!(
            compacted_lines
                .iter()
                .any(|l| l.contains("FieldUpdated") && l.contains("state")),
            "Expected FieldUpdated for state in Compacted section, got: {:?}",
            compacted_lines
        );
        assert!(
            compacted_lines
                .iter()
                .any(|l| l.contains("FieldUpdated") && l.contains("context.md")),
            "Expected FieldUpdated for context.md in Compacted section, got: {:?}",
            compacted_lines
        );
        assert!(
            compacted_lines
                .iter()
                .any(|l| l.contains("FieldUpdated") && l.contains("plan")),
            "Expected FieldUpdated for custom field 'plan' in Compacted section, got: {:?}",
            compacted_lines
        );
    }
}
