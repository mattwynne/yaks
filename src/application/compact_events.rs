// CompactEvents use case - compacts the event stream into a snapshot

use anyhow::Result;

use super::{Application, UseCase};

pub struct CompactEvents;

impl Default for CompactEvents {
    fn default() -> Self {
        Self
    }
}

impl CompactEvents {
    pub fn new() -> Self {
        Self
    }
}

impl UseCase for CompactEvents {
    fn execute(&self, app: &mut Application) -> Result<()> {
        use crate::domain::event_metadata::{EventMetadata, Timestamp};
        let metadata = EventMetadata::new(app.current_author(), Timestamp::now());
        app.event_store.compact(metadata)?;

        // Rebuild projection from the compacted event stream
        let all_events = app.event_store.get_all_events()?;
        app.event_bus.rebuild(&all_events)?;
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

        // Compact via handle
        app.handle(CompactEvents::new()).unwrap();

        // Yak should still exist after compaction
        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test-yak")).is_ok());

        // Event store should have a compacted event
        let events = EventStore::get_all_events(&event_store).unwrap();
        assert!(
            events.iter().any(|e| matches!(e, crate::domain::YakEvent::Compacted(_, _))),
            "Expected a Compacted event after compaction"
        );
    }
}
