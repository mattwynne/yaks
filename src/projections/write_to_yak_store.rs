use anyhow::Result;

use crate::domain::events::*;
use crate::domain::ports::{EventListener, WriteYakStore};
use crate::domain::slug::{Name, YakId};
use crate::domain::{YakEvent, NAME_FIELD, STATE_FIELD};

impl<T: WriteYakStore> EventListener for T {
    fn on_event(&mut self, event: &YakEvent) -> Result<()> {
        match event {
            YakEvent::Added(AddedEvent {
                name,
                id,
                parent_id,
            }) => {
                self.create_yak(name, id, parent_id.as_ref())?;
                // Use id for subsequent writes (storage resolves by id)
                let key = if id.as_str().is_empty() {
                    &YakId::from(name.as_str())
                } else {
                    id
                };
                self.write_field(key, STATE_FIELD, "todo")?;
                self.write_field(key, NAME_FIELD, name.as_str())?;
            }

            YakEvent::Removed(RemovedEvent { id }) => {
                self.delete_yak(id)?;
            }

            YakEvent::Moved(MovedEvent { id, new_parent }) => {
                self.reparent_yak(id, new_parent.as_ref())?;
            }

            YakEvent::FieldUpdated(FieldUpdatedEvent {
                id,
                field_name,
                content,
            }) => {
                if field_name == NAME_FIELD {
                    self.rename_yak(id, &Name::from(content.as_str()))?;
                } else {
                    self.write_field(id, field_name, content)?;
                }
            }
        }
        Ok(())
    }
}
