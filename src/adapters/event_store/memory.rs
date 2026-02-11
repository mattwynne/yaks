use anyhow::Result;
use std::sync::{Arc, Mutex};

use crate::domain::YakEvent;
use crate::ports::EventStore;

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

    fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .iter()
            .filter(|e| match e {
                YakEvent::Added { name: n } => n == name,
                YakEvent::Removed { name: n } => n == name,
                YakEvent::ContextUpdated { name: n, .. } => n == name,
                YakEvent::StateUpdated { name: n, .. } => n == name,
                YakEvent::Moved { old_name, .. } => old_name == name,
                YakEvent::FieldUpdated { name: n, .. } => n == name,
            })
            .cloned()
            .collect())
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        Ok(self.events.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_event_store() {
        let mut store = InMemoryEventStore::new();

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        store.append(&event).unwrap();
        let events = store.get_all_events().unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }
}
