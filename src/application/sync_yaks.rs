// SyncYaks use case - synchronizes yaks via event store sync

use anyhow::Result;

use super::{Application, UseCase};

pub struct SyncYaks;

impl Default for SyncYaks {
    fn default() -> Self {
        Self
    }
}

impl SyncYaks {
    pub fn new() -> Self {
        Self
    }
}

impl UseCase for SyncYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        app.sync_events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::domain::ports::EventStore;
    use crate::infrastructure::EventBus;

    #[test]
    fn test_sync_calls_event_store_sync() {
        let origin = InMemoryEventStore::new();
        let mut event_store = InMemoryEventStore::with_peer(&origin);
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

        app.handle(SyncYaks::new()).unwrap();
    }

    #[test]
    fn test_sync_exchanges_events_with_peer() {
        use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
        use crate::domain::events::AddedEvent;
        use crate::domain::slug::{Name, YakId};
        use crate::domain::YakEvent;

        // Add an event to the origin
        let mut origin = InMemoryEventStore::new();
        origin
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("peer yak"),
                    id: YakId::from("peer-yak-id"),
                    parent_id: None,
                },
                EventMetadata::new(
                    Author {
                        name: "test".to_string(),
                        email: "test@test.com".to_string(),
                    },
                    Timestamp::now(),
                ),
            ))
            .unwrap();

        let mut event_store = InMemoryEventStore::with_peer(&origin);
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

        app.handle(SyncYaks::new()).unwrap();

        // Local event store should now have the peer's event
        let events = EventStore::get_all_events(&event_store).unwrap();
        assert_eq!(
            events.len(),
            1,
            "Local should have pulled 1 event from peer"
        );
    }

    #[test]
    fn test_sync_fails_when_not_configured() {
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

        let result = app.handle(SyncYaks::new());
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Sync not configured");
    }
}
