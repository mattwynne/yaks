use anyhow::Result;

use crate::domain::YakEvent;

pub trait EventListener {
    fn on_event(&mut self, event: &YakEvent) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::AddedEvent;

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

        let event = YakEvent::Added(AddedEvent {
            name: "test".to_string(),
        });

        listener.on_event(&event).unwrap();

        assert_eq!(listener.events.len(), 1);
        assert_eq!(listener.events[0], event);
    }
}
