use anyhow::Result;

use crate::domain::events::*;
use crate::domain::{YakEvent, CONTEXT_FIELD, STATE_FIELD};
use crate::ports::WriteYakStore;

pub trait EventListener {
    fn on_event(&mut self, event: &YakEvent) -> Result<()>;
}

impl<T: WriteYakStore> EventListener for T {
    fn on_event(&mut self, event: &YakEvent) -> Result<()> {
        match event {
            YakEvent::Added(AddedEvent { name }) => {
                self.create_yak(name)?;
                self.write_field(name, STATE_FIELD, "todo")?;
            }

            YakEvent::Removed(RemovedEvent { name }) => {
                self.delete_yak(name)?;
            }

            YakEvent::Moved(MovedEvent { old_name, new_name }) => {
                self.rename_yak(old_name, new_name)?;
            }

            YakEvent::ContextUpdated(ContextUpdatedEvent { name, content }) => {
                self.write_field(name, CONTEXT_FIELD, content)?;
            }

            YakEvent::StateUpdated(StateUpdatedEvent { name, state }) => {
                self.write_field(name, STATE_FIELD, state)?;
            }

            YakEvent::FieldUpdated(FieldUpdatedEvent {
                name,
                field_name,
                content,
            }) => {
                self.write_field(name, field_name, content)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
