// ShowLog use case - displays the event log

use anyhow::Result;

use super::{Application, UseCase};
use crate::adapters::user_display::relative_time::format_relative;
use crate::adapters::views::{LogEntryView, NarrativeSpanView};
use crate::domain::narrative::NarrativeSpan;

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

        let resolve_name = |id: &str| -> String {
            use crate::domain::slug::YakId;
            app.store
                .get_yak(&YakId::from(id))
                .map(|y| y.name.to_string())
                .unwrap_or_else(|_| id.to_string())
        };

        let entries: Vec<LogEntryView> = events
            .iter()
            .rev()
            .map(|event| {
                let meta = event.metadata();
                let narrative = event.format_narrative(&meta.author.name, &resolve_name);
                let narrative_spans: Vec<NarrativeSpanView> = narrative
                    .iter()
                    .map(|span| match span {
                        NarrativeSpan::Plain(t) => NarrativeSpanView {
                            text: t.clone(),
                            bold: false,
                        },
                        NarrativeSpan::Highlight(t) => NarrativeSpanView {
                            text: t.clone(),
                            bold: true,
                        },
                    })
                    .collect();
                let timestamp = format_relative(meta.timestamp.as_epoch_secs());
                let event_id = meta.event_id.as_deref().unwrap_or("-").to_string();
                let commit_sha = meta.commit_sha.clone();

                LogEntryView {
                    narrative: narrative_spans,
                    relative_time: timestamp,
                    event_id,
                    commit_sha,
                }
            })
            .collect();

        app.display.show_log(&entries);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    struct TestWorkspace;

    impl crate::domain::ports::LocalWorkspacePort for TestWorkspace {
        fn is_yaks_gitignored(&self) -> anyhow::Result<bool> {
            Ok(true)
        }

        fn add_yaks_to_gitignore(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn commit_gitignore(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }
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
        let workspace = TestWorkspace;
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            Some(&reader),
            &auth,
        );

        app.handle(AddYak::new("test yak")).unwrap();
        buffer.clear();
        app.handle(ShowLog::new()).unwrap();

        let output = buffer.contents();
        assert!(
            output.contains("added test yak"),
            "Expected narrative 'added test yak' in output: {output:?}"
        );
        assert!(
            output.contains("event:"),
            "Expected 'event:' metadata line in output: {output:?}"
        );
        assert!(
            output.contains("────"),
            "Expected horizontal rule in output: {output:?}"
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
        let workspace = TestWorkspace;
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
    fn test_show_log_uses_narrative_format() {
        let mut event_store = InMemoryEventStore::new();
        let reader = event_store.clone();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();

        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            Some(&reader),
            &auth,
        );

        app.handle(AddYak::new("first yak")).unwrap();
        app.handle(AddYak::new("second yak")).unwrap();

        buffer.clear();

        app.handle(ShowLog::new()).unwrap();

        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        // Each event is 4 lines: narrative, timestamp, event: id, rule
        // 2 events = 4 + 4 = 8 lines
        assert_eq!(
            lines.len(),
            8,
            "Expected 8 lines for 2 events, got {}. Lines: {:?}",
            lines.len(),
            lines
        );
        // First event (newest - second yak added last)
        assert!(
            lines[0].contains("added second yak"),
            "Line 1: {:?}",
            lines[0]
        );
        assert!(lines[2].starts_with("event: "), "Line 3: {:?}", lines[2]);
        assert!(
            lines[3].contains("────"),
            "Line 4 should be rule: {:?}",
            lines[3]
        );
        // Second event (oldest - first yak added first)
        assert!(
            lines[4].contains("added first yak"),
            "Line 5: {:?}",
            lines[4]
        );
    }

    #[test]
    fn test_show_log_compacted_shows_single_narrative() {
        let mut event_store = InMemoryEventStore::new();
        let reader = event_store.clone();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();

        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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

        // Should show narrative, not expanded snapshots
        assert!(
            output.contains("compacted the event stream"),
            "Expected compacted narrative in output: {output:?}"
        );
        // Should NOT show expanded snapshot details
        assert!(
            !output.contains("FieldUpdated"),
            "Should not contain FieldUpdated expansion: {output:?}"
        );
        assert!(
            !output.contains("        Added:"),
            "Should not contain indented Added: {output:?}"
        );
    }
}
