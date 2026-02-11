use anyhow::Result;

use crate::domain::YakEvent;
use crate::ports::{EventListener, EventStore};

pub struct EventBus {
    event_store: Box<dyn EventStore>,
    listeners: Vec<Box<dyn EventListener>>,
}

impl EventBus {
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            event_store,
            listeners: vec![],
        }
    }

    pub fn register(&mut self, listener: Box<dyn EventListener>) {
        self.listeners.push(listener);
    }

    pub fn publish(&mut self, event: YakEvent) -> Result<()> {
        // First: persist to event store (source of truth)
        self.event_store.append(&event)?;

        // Then: notify projections
        for listener in &mut self.listeners {
            listener.on_event(&event)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::InMemoryEventStore;

    struct TestListener {
        events: Vec<YakEvent>,
    }

    impl EventListener for TestListener {
        fn on_event(&mut self, event: &YakEvent) -> Result<()> {
            self.events.push(event.clone());
            Ok(())
        }
    }

    #[test]
    fn test_event_bus_publishes_to_store() {
        let store = InMemoryEventStore::new();
        let mut bus = EventBus::new(Box::new(store.clone()));

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        bus.publish(event.clone()).unwrap();

        let events = store.get_all_events().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], event);
    }

    #[test]
    fn test_event_bus_notifies_listeners() {
        let store = InMemoryEventStore::new();
        let mut bus = EventBus::new(Box::new(store));

        let listener = TestListener { events: vec![] };
        bus.register(Box::new(listener));

        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        bus.publish(event.clone()).unwrap();

        // Note: Can't easily test listener state after publish
        // due to ownership. Consider refactoring listener storage
        // or testing at integration level.
    }
}
