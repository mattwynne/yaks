use anyhow::Result;

use crate::domain::events::*;
use crate::domain::ports::{EventListener, WriteYakStore};
use crate::domain::slug::{Name, YakId};
use crate::domain::{YakEvent, METADATA_FIELD, NAME_FIELD, STATE_FIELD};

impl<T: WriteYakStore> EventListener for T {
    fn clear(&mut self) -> Result<()> {
        self.clear_all()
    }

    fn on_event(&mut self, event: &YakEvent) -> Result<()> {
        let result = apply_event(self, event);
        if let Err(e) = &result {
            if e.to_string().contains("not found") {
                return Ok(());
            }
        }
        result
    }
}

fn apply_event<T: WriteYakStore>(store: &mut T, event: &YakEvent) -> Result<()> {
    match event {
        YakEvent::Added(
            AddedEvent {
                name,
                id,
                parent_id,
            },
            metadata,
        ) => {
            store.create_yak(name, id, parent_id.as_ref())?;
            let key = if id.as_str().is_empty() {
                &YakId::from(name.as_str())
            } else {
                id
            };
            store.write_field(key, STATE_FIELD, "todo")?;
            store.write_field(key, NAME_FIELD, name.as_str())?;
            let metadata_json = serde_json::json!({
                "created_by": {
                    "name": metadata.author.name,
                    "email": metadata.author.email
                },
                "created_at": metadata.timestamp.as_epoch_secs()
            });
            store.write_field(key, METADATA_FIELD, &metadata_json.to_string())?;
        }

        YakEvent::Removed(RemovedEvent { id }, _) => {
            store.delete_yak(id)?;
        }

        YakEvent::Moved(MovedEvent { id, new_parent }, _) => {
            store.reparent_yak(id, new_parent.as_ref())?;
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
                store.rename_yak(id, &Name::from(content.as_str()))?;
            } else {
                store.write_field(id, field_name, content)?;
            }
        }
    }
    Ok(())
}
