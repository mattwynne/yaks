use anyhow::Result;

use crate::domain::YakEvent;

pub trait EventListener {
    fn on_event(&mut self, event: &YakEvent) -> Result<()>;

    /// Clear all state, preparing for a full replay of events.
    /// Default implementation is a no-op for listeners that
    /// don't need explicit clearing.
    fn clear(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::EventMetadata;
    use crate::domain::events::AddedEvent;
    use crate::domain::slug::{Name, YakId};

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
    fn test_event_listener_receives_events() {
        let mut listener = TestListener { events: vec![] };

        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );

        listener.on_event(&event).unwrap();

        assert_eq!(listener.events.len(), 1);
        assert_eq!(listener.events[0], event);
    }
}
