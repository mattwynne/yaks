use anyhow::Result;

use crate::domain::events::*;
use crate::domain::ports::{EventListener, WriteYakStore};
use crate::domain::{YakEvent, CONTEXT_FIELD, STATE_FIELD};

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
