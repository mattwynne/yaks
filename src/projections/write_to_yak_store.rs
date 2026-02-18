use anyhow::Result;

use crate::domain::events::*;
use crate::domain::ports::{EventListener, WriteYakStore};
use crate::domain::slug::{Name, YakId};
use crate::domain::{YakEvent, METADATA_FIELD, NAME_FIELD, STATE_FIELD};

impl<T: WriteYakStore> EventListener for T {
    fn on_event(&mut self, event: &YakEvent) -> Result<()> {
        match event {
            YakEvent::Added(
                AddedEvent {
                    name,
                    id,
                    parent_id,
                },
                metadata,
            ) => {
                self.create_yak(name, id, parent_id.as_ref())?;
                // Use id for subsequent writes (storage resolves by id)
                let key = if id.as_str().is_empty() {
                    &YakId::from(name.as_str())
                } else {
                    id
                };
                self.write_field(key, STATE_FIELD, "todo")?;
                self.write_field(key, NAME_FIELD, name.as_str())?;
                let metadata_json = serde_json::json!({
                    "created_by": {
                        "name": metadata.author.name,
                        "email": metadata.author.email
                    },
                    "created_at": metadata.timestamp.as_epoch_secs()
                });
                self.write_field(key, METADATA_FIELD, &metadata_json.to_string())?;
            }

            YakEvent::Removed(RemovedEvent { id }, _) => {
                self.delete_yak(id)?;
            }

            YakEvent::Moved(MovedEvent { id, new_parent }, _) => {
                self.reparent_yak(id, new_parent.as_ref())?;
            }

            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id,
                    field_name,
                    content,
                },
                _,
            ) => {
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
