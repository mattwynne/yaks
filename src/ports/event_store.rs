use anyhow::Result;
use crate::domain::YakEvent;

pub trait EventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()>;
    fn get_events(&self, name: &str) -> Result<Vec<YakEvent>>;
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
}

#[cfg(test)]
mod tests {
    use super::*;

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
            Ok(self.events.clone())
        }
    }

    #[test]
    fn test_event_store_append() {
        let mut store = InMemoryEventStore { events: vec![] };

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        store.append(&event).unwrap();
        let events = store.get_all_events().unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn test_event_store_get_events_by_name() {
        let mut store = InMemoryEventStore { events: vec![] };

        store
            .append(&YakEvent::Added {
                name: "test1".to_string(),
            })
            .unwrap();
        store
            .append(&YakEvent::Added {
                name: "test2".to_string(),
            })
            .unwrap();
        store
            .append(&YakEvent::ContextUpdated {
                name: "test1".to_string(),
                content: "content".to_string(),
            })
            .unwrap();

        let events = store.get_events("test1").unwrap();

        assert_eq!(events.len(), 2);
    }
}
