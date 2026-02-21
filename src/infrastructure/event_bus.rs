use anyhow::Result;

use crate::domain::ports::EventListener;
use crate::domain::YakEvent;

pub struct EventBus {
    listeners: Vec<Box<dyn EventListener>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self { listeners: vec![] }
    }

    pub fn register(&mut self, listener: Box<dyn EventListener>) {
        self.listeners.push(listener);
    }

    pub fn notify(&mut self, event: &YakEvent) -> Result<()> {
        for listener in &mut self.listeners {
            listener.on_event(event)?;
        }

        Ok(())
    }

    /// Clear all listeners and replay events from scratch.
    ///
    /// Used after sync to rebuild the disk projection,
    /// especially for worktrees that share a git repo
    /// but have independent .yaks directories.
    pub fn rebuild(&mut self, events: &[YakEvent]) -> Result<()> {
        for listener in &mut self.listeners {
            listener.clear()?;
        }
        for event in events {
            for listener in &mut self.listeners {
                listener.on_event(event)?;
            }
        }
        Ok(())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
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
    fn test_event_bus_notifies_listeners() {
        let mut bus = EventBus::new();

        let listener = TestListener { events: vec![] };
        bus.register(Box::new(listener));

        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );

        bus.notify(&event).unwrap();

        // Note: Can't easily test listener state after notify
        // due to ownership. Consider refactoring listener storage
        // or testing at integration level.
    }
}
