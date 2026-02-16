use anyhow::Result;

use crate::domain::events::*;
use crate::domain::ports::{EventListener, WriteYakStore};
use crate::domain::{YakEvent, CONTEXT_FIELD, NAME_FIELD, STATE_FIELD};

impl<T: WriteYakStore> EventListener for T {
    fn on_event(&mut self, event: &YakEvent) -> Result<()> {
        match event {
            YakEvent::Added(AddedEvent {
                name,
                id,
                parent_id,
            }) => {
                self.create_yak(name, id, parent_id.as_deref())?;
                // Use id for subsequent writes (storage resolves by id)
                let key = if id.is_empty() {
                    name.as_str()
                } else {
                    id.as_str()
                };
                self.write_field(key, STATE_FIELD, "todo")?;
                self.write_field(key, NAME_FIELD, name)?;
            }

            YakEvent::Removed(RemovedEvent { id }) => {
                self.delete_yak(id)?;
            }

            YakEvent::Moved(MovedEvent { id, new_parent }) => {
                self.reparent_yak(id, new_parent.as_deref())?;
            }

            YakEvent::Renamed(RenamedEvent { id, new_name }) => {
                self.rename_yak(id, new_name)?;
            }

            YakEvent::ContextUpdated(ContextUpdatedEvent { id, content }) => {
                self.write_field(id, CONTEXT_FIELD, content)?;
            }

            YakEvent::StateUpdated(StateUpdatedEvent { id, state }) => {
                self.write_field(id, STATE_FIELD, state)?;
            }

            YakEvent::FieldUpdated(FieldUpdatedEvent {
                id,
                field_name,
                content,
            }) => {
                self.write_field(id, field_name, content)?;
            }
        }
        Ok(())
    }
}
