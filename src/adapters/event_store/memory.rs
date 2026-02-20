use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::domain::ports::{EventStore, EventStoreReader};
use crate::domain::{Yak, YakEvent};

#[derive(Clone)]
pub struct InMemoryEventStore {
    events: Arc<Mutex<Vec<YakEvent>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .iter()
            .filter(|e| e.yak_id() == name)
            .cloned()
            .collect())
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for InMemoryEventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        if let Some(id) = &event.metadata().event_id {
            if events
                .iter()
                .any(|e| e.metadata().event_id.as_deref() == Some(id))
            {
                return Ok(());
            }
        }
        let event = if event.metadata().event_id.is_none() {
            let mut metadata = event.metadata().clone();
            metadata.event_id = Some(uuid::Uuid::new_v4().to_string());
            event.clone().with_metadata(metadata)
        } else {
            event.clone()
        };
        events.push(event);
        Ok(())
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        Ok(self.events.lock().unwrap().clone())
    }

    fn reset_from_snapshot(&mut self, _yaks: &[Yak]) -> Result<usize> {
        Ok(0)
    }
}

impl EventStoreReader for InMemoryEventStore {
    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        EventStore::get_all_events(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::EventMetadata;
    use crate::domain::events::AddedEvent;
    use crate::domain::slug::{Name, YakId};

    #[test]
    fn test_in_memory_event_store() {
        let mut store = InMemoryEventStore::new();

        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );

        store.append(&event).unwrap();
        let events = EventStore::get_all_events(&store).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].yak_id(), "");
    }

    #[test]
    fn test_get_all_events_empty_store() {
        let store = InMemoryEventStore::new();
        let events = EventStore::get_all_events(&store).unwrap();

        assert_eq!(events.len(), 0);
        assert!(events.is_empty());
    }

    #[test]
    fn test_reset_from_snapshot_returns_zero() {
        let mut store = InMemoryEventStore::new();
        let result = store.reset_from_snapshot(&[]).unwrap();

        assert_eq!(result, 0);
    }
}
