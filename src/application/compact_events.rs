// CompactEvents use case - compacts the event stream into a snapshot

use anyhow::Result;

use super::{Application, UseCase};

#[derive(Default)]
pub struct CompactEvents {
    /// Skip the confirmation prompt
    skip_confirm: bool,
}

impl CompactEvents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Skip the confirmation prompt (equivalent to --yes flag)
    pub fn with_skip_confirm(mut self, skip: bool) -> Self {
        self.skip_confirm = skip;
        self
    }
}

impl UseCase for CompactEvents {
    fn execute(&self, app: &mut Application) -> Result<()> {
        use crate::domain::event_metadata::{EventMetadata, Timestamp};

        // 1. Auto-sync first
        match app.sync_events() {
            Ok(()) => {}
            Err(e) => {
                app.display.warn(&format!("sync failed: {}", e));
            }
        }

        // 2. Confirmation prompt (unless skip_confirm)
        if !self.skip_confirm {
            let confirmed = app.input.confirm(
                "Warning: collaborators with unsynced local events \
                 will lose them. Ask them to run 'yx sync' first.\n\
                 Proceed?",
            )?;
            if !confirmed {
                return Ok(());
            }
        }

        // 3. Compact the event store
        let metadata = EventMetadata::new(app.current_author(), Timestamp::now());
        app.event_store.compact(metadata)?;

        // 4. Rebuild projection from the compacted event stream
        let all_events = app.event_store.get_all_events()?;
        app.event_bus.rebuild(&all_events)?;

        // 5. Report success
        app.display.info("Compacted event stream.");

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
    use crate::application::AddYak;
    use crate::domain::ports::{EventStore, ReadYakStore};
    use crate::domain::slug::YakId;
    use crate::infrastructure::EventBus;

    #[test]
    fn test_compact_events_via_handle() {
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

        // Add a yak first so there's something to compact
        app.handle(AddYak::new("test-yak")).unwrap();

        // Compact via handle (InMemoryInput defaults to confirm=true)
        app.handle(CompactEvents::new()).unwrap();

        // Yak should still exist after compaction
        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test-yak")).is_ok());

        // Event store should have a compacted event
        let events = EventStore::get_all_events(&event_store).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, crate::domain::YakEvent::Compacted(_, _))),
            "Expected a Compacted event after compaction"
        );
    }

    #[test]
    fn test_compact_events_skips_when_not_confirmed() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        input.set_confirm(false); // User declines
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

        // Add a yak first
        app.handle(AddYak::new("test-yak")).unwrap();

        // Compact should be a no-op when user declines
        app.handle(CompactEvents::new()).unwrap();

        // Event store should NOT have a compacted event
        let events = EventStore::get_all_events(&event_store).unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, crate::domain::YakEvent::Compacted(_, _))),
            "Should not compact when user declines"
        );
    }

    #[test]
    fn test_compact_events_with_skip_confirm() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        input.set_confirm(false); // Would decline, but skip_confirm bypasses
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

        app.handle(AddYak::new("test-yak")).unwrap();
        app.handle(CompactEvents::new().with_skip_confirm(true))
            .unwrap();

        let events = EventStore::get_all_events(&event_store).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, crate::domain::YakEvent::Compacted(_, _))),
            "Should compact when skip_confirm is true"
        );
    }
}
