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
        self.events.lock().unwrap().push(event.clone());
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
    use crate::domain::events::AddedEvent;
    use crate::domain::slug::{Name, YakId};

    #[test]
    fn test_in_memory_event_store() {
        let mut store = InMemoryEventStore::new();

        let event = YakEvent::Added(AddedEvent {
            name: Name::from("test"),
            id: YakId::from(""),
            parent_id: None,
        });

        store.append(&event).unwrap();
        let events = EventStore::get_all_events(&store).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }
}
