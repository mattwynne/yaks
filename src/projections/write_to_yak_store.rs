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
                self.create_yak(
                    name.as_str(),
                    id.as_str(),
                    parent_id.as_ref().map(|p| p.as_str()),
                )?;
                // Use id for subsequent writes (storage resolves by id)
                let key = if id.as_str().is_empty() {
                    name.as_str()
                } else {
                    id.as_str()
                };
                self.write_field(key, STATE_FIELD, "todo")?;
                self.write_field(key, NAME_FIELD, name.as_str())?;
            }

            YakEvent::Removed(RemovedEvent { id }) => {
                self.delete_yak(id.as_str())?;
            }

            YakEvent::Moved(MovedEvent { id, new_parent }) => {
                self.reparent_yak(id.as_str(), new_parent.as_ref().map(|p| p.as_str()))?;
            }

            YakEvent::Renamed(RenamedEvent { id, new_name }) => {
                self.rename_yak(id.as_str(), new_name.as_str())?;
            }

            YakEvent::ContextUpdated(ContextUpdatedEvent { id, content }) => {
                self.write_field(id.as_str(), CONTEXT_FIELD, content)?;
            }

            YakEvent::StateUpdated(StateUpdatedEvent { id, state }) => {
                self.write_field(id.as_str(), STATE_FIELD, state)?;
            }

            YakEvent::FieldUpdated(FieldUpdatedEvent {
                id,
                field_name,
                content,
            }) => {
                self.write_field(id.as_str(), field_name, content)?;
            }
        }
        Ok(())
    }
}
