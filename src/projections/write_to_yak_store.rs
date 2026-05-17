use anyhow::Result;

use crate::domain::events::*;
use crate::domain::ports::{EventListener, WriteYakStore};
use crate::domain::slug::{Name, YakId};
use crate::domain::yak_map::MANUAL_BLOCKER_FIELD;
use crate::domain::{YakEvent, CONTEXT_FIELD, CREATED_FIELD, NAME_FIELD, STATE_FIELD};

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

/// Map legacy field names to their current dot-prefixed equivalents.
/// Old events may have been written with bare names before the convention
/// was established (e.g. "tags" instead of ".tags").
fn migrate_field_name(field_name: &str) -> &str {
    match field_name {
        "tags" => ".tags",
        _ => field_name,
    }
}

fn apply_field_updated_event<T: WriteYakStore>(
    store: &mut T,
    id: &YakId,
    field_name: &str,
    content: &str,
) -> Result<()> {
    let actual_name = migrate_field_name(field_name);
    if actual_name == NAME_FIELD {
        store.rename_yak(id, &Name::from(content))?;
    } else if actual_name == STATE_FIELD && content == "blocked" {
        store.write_field(id, STATE_FIELD, "todo")?;
        store.write_field(
            id,
            MANUAL_BLOCKER_FIELD,
            crate::domain::yak_map::MIGRATED_BLOCKED_REASON,
        )?;
    } else {
        store.write_field(id, actual_name, content)?;
    }
    Ok(())
}

fn apply_manual_blocker_event<T: WriteYakStore>(store: &mut T, event: &YakEvent) -> Result<bool> {
    match event {
        YakEvent::ManualBlockerAdded(e, _) => {
            store.write_field(&e.target, MANUAL_BLOCKER_FIELD, &e.reason)?;
            Ok(true)
        }
        YakEvent::ManualBlockerUpdated(e, _) => {
            store.write_field(&e.target, MANUAL_BLOCKER_FIELD, &e.reason)?;
            Ok(true)
        }
        YakEvent::ManualBlockerRemoved(e, _) => {
            store.write_field(&e.target, MANUAL_BLOCKER_FIELD, "")?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn apply_event<T: WriteYakStore>(store: &mut T, event: &YakEvent) -> Result<()> {
    if apply_manual_blocker_event(store, event)? {
        return Ok(());
    }

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
            store.write_field(key, CREATED_FIELD, &metadata_json.to_string())?;
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
        ) => apply_field_updated_event(store, id, field_name, content)?,

        YakEvent::BlockerAdded(_, _)
        | YakEvent::BlockerUpdated(_, _)
        | YakEvent::BlockerRemoved(_, _)
        | YakEvent::ManualBlockerAdded(_, _)
        | YakEvent::ManualBlockerUpdated(_, _)
        | YakEvent::ManualBlockerRemoved(_, _) => {}

        YakEvent::Compacted(snapshots, _, _) | YakEvent::Migrated(snapshots, _, _) => {
            store.clear_all()?;
            for snap in snapshots {
                store.create_yak(&snap.name, &snap.id, snap.parent_id.as_ref())?;
                store.write_field(&snap.id, STATE_FIELD, &snap.state.to_string())?;
                store.write_field(&snap.id, NAME_FIELD, snap.name.as_str())?;
                if let Some(ref ctx) = snap.context {
                    if !ctx.is_empty() {
                        store.write_field(&snap.id, CONTEXT_FIELD, ctx)?;
                    }
                }
                let metadata_json = serde_json::json!({
                    "created_by": {
                        "name": snap.created_by.name,
                        "email": snap.created_by.email
                    },
                    "created_at": snap.created_at.as_epoch_secs()
                });
                store.write_field(&snap.id, CREATED_FIELD, &metadata_json.to_string())?;
                for (field_name, content) in &snap.fields {
                    let actual_name = migrate_field_name(field_name);
                    store.write_field(&snap.id, actual_name, content)?;
                }
            }
        }
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
    fn compacted_event_rebuilds_store_from_snapshots() {
        use crate::domain::event_metadata::{Author, Timestamp};
        use crate::domain::yak_snapshot::YakSnapshot;
        use std::collections::HashMap;

        let storage = InMemoryStorage::new();
        let mut listener: Box<dyn EventListener> = Box::new(storage.clone());

        // Pre-existing yak that should be cleared
        let old_event = added_event("old yak", "old-a1b2");
        listener.on_event(&old_event).unwrap();
        assert_eq!(storage.list_yaks().unwrap().len(), 1);

        // Compacted event with different yaks
        let snapshots = vec![
            YakSnapshot {
                id: YakId::from("tea-a1b2"),
                name: Name::from("make the tea"),
                parent_id: None,
                state: crate::domain::YakState::Wip,
                context: Some("use the good teapot".to_string()),
                fields: HashMap::new(),
                tags: vec![],
                created_by: Author {
                    name: "alice".into(),
                    email: "alice@example.com".into(),
                },
                created_at: Timestamp(1000),
            },
            YakSnapshot {
                id: YakId::from("biscuits-c3d4"),
                name: Name::from("buy biscuits"),
                parent_id: Some(YakId::from("tea-a1b2")),
                state: crate::domain::YakState::Todo,
                context: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: Author {
                    name: "alice".into(),
                    email: "alice@example.com".into(),
                },
                created_at: Timestamp(1000),
            },
        ];

        let compacted = YakEvent::Compacted(snapshots, vec![], EventMetadata::default_legacy());
        listener.on_event(&compacted).unwrap();

        let yaks = storage.list_yaks().unwrap();
        assert_eq!(yaks.len(), 2, "Should have exactly the 2 snapshot yaks");
        assert!(
            !yaks.iter().any(|y| y.name == Name::from("old yak")),
            "Old yak should be cleared"
        );

        let tea = yaks
            .iter()
            .find(|y| y.id == YakId::from("tea-a1b2"))
            .unwrap();
        assert_eq!(tea.state, crate::domain::YakState::Wip);
        assert_eq!(
            tea.context,
            Some("use the good teapot".to_string()),
            "Context should be preserved through compaction"
        );

        let biscuits = yaks
            .iter()
            .find(|y| y.id == YakId::from("biscuits-c3d4"))
            .unwrap();
        assert_eq!(
            biscuits.parent_id.as_ref().unwrap(),
            &YakId::from("tea-a1b2")
        );
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

    #[test]
    fn legacy_tags_field_name_is_migrated_to_dot_tags() {
        let storage = InMemoryStorage::new();
        let mut listener: Box<dyn EventListener> = Box::new(storage.clone());

        // Add a yak
        listener
            .on_event(&added_event("my yak", "my-yak-a1b2"))
            .unwrap();

        // Simulate a legacy event with bare "tags" field name
        let legacy_event = YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id: YakId::from("my-yak-a1b2"),
                field_name: "tags".to_string(),
                content: "v1\nurgent".to_string(),
            },
            EventMetadata::default_legacy(),
        );
        listener.on_event(&legacy_event).unwrap();

        // Tags should be accessible via the yak view
        let yak = storage.get_yak(&YakId::from("my-yak-a1b2")).unwrap();
        assert_eq!(yak.tags, vec!["v1".to_string(), "urgent".to_string()]);
        // Should NOT appear as a custom field named "tags"
        assert!(!yak.fields.contains_key("tags"));
    }

    #[test]
    fn legacy_blocked_state_projects_as_todo_with_manual_blocker() {
        let storage = InMemoryStorage::new();
        let mut listener: Box<dyn EventListener> = Box::new(storage.clone());

        listener
            .on_event(&added_event("blocked yak", "blocked-yak-a1b2"))
            .unwrap();
        listener
            .on_event(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("blocked-yak-a1b2"),
                    field_name: STATE_FIELD.to_string(),
                    content: "blocked".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let yak = storage.get_yak(&YakId::from("blocked-yak-a1b2")).unwrap();
        assert_eq!(yak.state, crate::domain::YakState::Todo);
        assert_eq!(
            yak.fields.get(MANUAL_BLOCKER_FIELD).map(String::as_str),
            Some(crate::domain::yak_map::MIGRATED_BLOCKED_REASON)
        );
    }

    #[test]
    fn manual_blocker_events_update_projection_field() {
        let storage = InMemoryStorage::new();
        let mut listener: Box<dyn EventListener> = Box::new(storage.clone());
        let id = YakId::from("blocked-yak-a1b2");

        listener
            .on_event(&added_event("blocked yak", id.as_str()))
            .unwrap();
        listener
            .on_event(&YakEvent::ManualBlockerAdded(
                ManualBlockerAddedEvent {
                    target: id.clone(),
                    reason: "waiting on vendor".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();
        assert_eq!(
            storage
                .get_yak(&id)
                .unwrap()
                .fields
                .get(MANUAL_BLOCKER_FIELD)
                .map(String::as_str),
            Some("waiting on vendor")
        );

        listener
            .on_event(&YakEvent::ManualBlockerUpdated(
                ManualBlockerUpdatedEvent {
                    target: id.clone(),
                    reason: "waiting on review".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();
        assert_eq!(
            storage
                .get_yak(&id)
                .unwrap()
                .fields
                .get(MANUAL_BLOCKER_FIELD)
                .map(String::as_str),
            Some("waiting on review")
        );

        listener
            .on_event(&YakEvent::ManualBlockerRemoved(
                ManualBlockerRemovedEvent { target: id.clone() },
                EventMetadata::default_legacy(),
            ))
            .unwrap();
        assert_eq!(
            storage
                .get_yak(&id)
                .unwrap()
                .fields
                .get(MANUAL_BLOCKER_FIELD)
                .map(String::as_str),
            Some("")
        );
    }

    #[test]
    fn compacted_event_migrates_legacy_tags_field() {
        use crate::domain::event_metadata::{Author, Timestamp};
        use crate::domain::yak_snapshot::YakSnapshot;
        use std::collections::HashMap;

        let storage = InMemoryStorage::new();
        let mut listener: Box<dyn EventListener> = Box::new(storage.clone());

        // Snapshot with legacy "tags" field name in fields map
        let mut fields = HashMap::new();
        fields.insert("tags".to_string(), "v1\nv2".to_string());

        let snapshots = vec![YakSnapshot {
            id: YakId::from("yak-a1b2"),
            name: Name::from("my yak"),
            parent_id: None,
            state: crate::domain::YakState::Todo,
            context: None,
            fields,
            tags: vec![],
            created_by: Author {
                name: "alice".into(),
                email: "alice@example.com".into(),
            },
            created_at: Timestamp(1000),
        }];

        let compacted = YakEvent::Compacted(snapshots, vec![], EventMetadata::default_legacy());
        listener.on_event(&compacted).unwrap();

        let yak = storage.get_yak(&YakId::from("yak-a1b2")).unwrap();
        assert_eq!(yak.tags, vec!["v1".to_string(), "v2".to_string()]);
        assert!(!yak.fields.contains_key("tags"));
    }
}
