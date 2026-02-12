use crate::domain::YakEvent;
use anyhow::Result;

pub trait EventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()>;
    #[allow(dead_code)]
    fn get_events(&self, name: &str) -> Result<Vec<YakEvent>>;
    #[allow(dead_code)]
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::{AddedEvent, ContextUpdatedEvent};

    struct InMemoryEventStore {
        events: Vec<YakEvent>,
    }

    impl EventStore for InMemoryEventStore {
        fn append(&mut self, event: &YakEvent) -> Result<()> {
            self.events.push(event.clone());
            Ok(())
        }

        fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
            Ok(self
                .events
                .iter()
                .filter(|e| e.yak_name() == name)
                .cloned()
                .collect())
        }

        fn get_all_events(&self) -> Result<Vec<YakEvent>> {
            Ok(self.events.clone())
        }
    }

    #[test]
    fn test_event_store_append() {
        let mut store = InMemoryEventStore { events: vec![] };

        let event = YakEvent::Added(AddedEvent {
            name: "test".to_string(),
        });

        store.append(&event).unwrap();
        let events = store.get_all_events().unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn test_event_store_get_events_by_name() {
        let mut store = InMemoryEventStore { events: vec![] };

        store
            .append(&YakEvent::Added(AddedEvent {
                name: "test1".to_string(),
            }))
            .unwrap();
        store
            .append(&YakEvent::Added(AddedEvent {
                name: "test2".to_string(),
            }))
            .unwrap();
        store
            .append(&YakEvent::ContextUpdated(ContextUpdatedEvent {
                name: "test1".to_string(),
                content: "content".to_string(),
            }))
            .unwrap();

        let events = store.get_events("test1").unwrap();

        assert_eq!(events.len(), 2);
    }
}
