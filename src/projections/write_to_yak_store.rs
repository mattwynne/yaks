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
            let msg = e.to_string();
            // Tolerate stale references and duplicate events during
            // rebuild/sync — events are immutable facts, so the
            // projection must be idempotent.
            if msg.contains("not found") || msg.contains("already exists") {
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

        // Compacted events are expanded by get_all_events() and should
        // never reach the projection. Ignore if encountered.
        YakEvent::Compacted(_, _) => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::InMemoryStorage;
    use crate::domain::event_metadata::EventMetadata;
    use crate::domain::ports::{EventListener, ReadYakStore};

    fn added_event(name: &str, id: &str) -> YakEvent {
        YakEvent::Added(
            AddedEvent {
                name: Name::from(name),
                id: YakId::from(id),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        )
    }

    #[test]
    fn duplicate_added_event_is_tolerated() {
        let storage = InMemoryStorage::new();
        let mut listener: Box<dyn EventListener> = Box::new(storage.clone());

        let event = added_event("make the tea", "tea-id");

        listener.on_event(&event).unwrap();
        listener.on_event(&event).unwrap(); // duplicate

        let yaks = storage.list_yaks().unwrap();
        assert_eq!(yaks.len(), 1);
        assert_eq!(yaks[0].name, Name::from("make the tea"));
    }

    #[test]
    fn removed_event_for_missing_yak_is_tolerated() {
        let storage = InMemoryStorage::new();
        let mut listener: Box<dyn EventListener> = Box::new(storage.clone());

        let event = YakEvent::Removed(
            RemovedEvent {
                id: YakId::from("nonexistent"),
            },
            EventMetadata::default_legacy(),
        );

        listener.on_event(&event).unwrap(); // no yak to remove
    }
}
