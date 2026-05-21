use serde::Serialize;
use serde_json::{json, Value};

use crate::domain::events::{Blocker, BlockerSource};
use crate::domain::narrative::to_plain_text;
use crate::domain::YakEvent;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventNotification {
    pub event_id: String,
    pub event_type: String,
    pub yak_id: String,
    pub yak_name: String,
    pub timestamp: i64,
    pub author: String,
    pub narrative: String,
    pub event: Value,
}

impl EventNotification {
    pub fn from_event(event: &YakEvent, resolve_name: &dyn Fn(&str) -> String) -> Self {
        let metadata = event.metadata();
        let yak_id = event.yak_id().to_string();
        let yak_name = if yak_id.is_empty() {
            String::new()
        } else {
            resolve_name(&yak_id)
        };
        let narrative = to_plain_text(&event.format_narrative(&metadata.author.name, resolve_name));

        Self {
            event_id: metadata.event_id.clone().unwrap_or_else(|| "-".to_string()),
            event_type: yak_event_type(event).to_string(),
            yak_id,
            yak_name,
            timestamp: metadata.timestamp.as_epoch_secs(),
            author: metadata.author.name.clone(),
            narrative,
            event: event_payload(event),
        }
    }

    pub fn to_json_line(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

pub fn event_notification_json_line(
    event: &YakEvent,
    resolve_name: &dyn Fn(&str) -> String,
) -> serde_json::Result<String> {
    EventNotification::from_event(event, resolve_name).to_json_line()
}

pub fn yak_event_type(event: &YakEvent) -> &'static str {
    match event {
        YakEvent::Added(_, _) => "Added",
        YakEvent::Removed(_, _) => "Removed",
        YakEvent::Moved(_, _) => "Moved",
        YakEvent::FieldUpdated(_, _) => "FieldUpdated",
        YakEvent::BlockerAdded(_, _) => "BlockerAdded",
        YakEvent::BlockerUpdated(_, _) => "BlockerUpdated",
        YakEvent::BlockerRemoved(_, _) => "BlockerRemoved",
        YakEvent::ManualBlockerAdded(_, _) => "ManualBlockerAdded",
        YakEvent::ManualBlockerUpdated(_, _) => "ManualBlockerUpdated",
        YakEvent::ManualBlockerRemoved(_, _) => "ManualBlockerRemoved",
        YakEvent::Compacted(_, _) => "Compacted",
        YakEvent::Migrated(_, _) => "Migrated",
    }
}

fn event_payload(event: &YakEvent) -> Value {
    match event {
        YakEvent::Added(e, _) => json!({
            "type": "Added",
            "id": e.id.as_str(),
            "name": e.name.as_ref(),
            "parent_id": e.parent_id.as_ref().map(|id| id.as_str()),
        }),
        YakEvent::Removed(e, _) => json!({
            "type": "Removed",
            "id": e.id.as_str(),
        }),
        YakEvent::Moved(e, _) => json!({
            "type": "Moved",
            "id": e.id.as_str(),
            "new_parent": e.new_parent.as_ref().map(|id| id.as_str()),
        }),
        YakEvent::FieldUpdated(e, _) => json!({
            "type": "FieldUpdated",
            "id": e.id.as_str(),
            "field_name": e.field_name,
        }),
        YakEvent::BlockerAdded(e, _) => json!({
            "type": "BlockerAdded",
            "target": e.target.as_str(),
            "blocker": blocker_payload(&e.blocker),
        }),
        YakEvent::BlockerUpdated(e, _) => json!({
            "type": "BlockerUpdated",
            "target": e.target.as_str(),
            "blocker": blocker_payload(&e.blocker),
        }),
        YakEvent::BlockerRemoved(e, _) => json!({
            "type": "BlockerRemoved",
            "target": e.target.as_str(),
            "source": blocker_source_payload(&e.source),
        }),
        YakEvent::ManualBlockerAdded(e, _) => json!({
            "type": "ManualBlockerAdded",
            "target": e.target.as_str(),
            "reason": e.reason,
        }),
        YakEvent::ManualBlockerUpdated(e, _) => json!({
            "type": "ManualBlockerUpdated",
            "target": e.target.as_str(),
            "reason": e.reason,
        }),
        YakEvent::ManualBlockerRemoved(e, _) => json!({
            "type": "ManualBlockerRemoved",
            "target": e.target.as_str(),
        }),
        YakEvent::Compacted(snapshot, _) => json!({
            "type": "Compacted",
            "yak_count": snapshot.yak_count(),
        }),
        YakEvent::Migrated(snapshot, _) => json!({
            "type": "Migrated",
            "yak_count": snapshot.yak_count(),
        }),
    }
}

fn blocker_payload(blocker: &Blocker) -> Value {
    json!({
        "source": blocker_source_payload(&blocker.source),
        "reason": blocker.reason,
    })
}

fn blocker_source_payload(source: &BlockerSource) -> Value {
    match source {
        BlockerSource::Yak(id) => json!({ "kind": "yak", "id": id.as_str() }),
        BlockerSource::Manual => json!({ "kind": "manual" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
    use crate::domain::events::{
        AddedEvent, BlockerRemovedEvent, BlockerSource, FieldUpdatedEvent, MovedEvent, RemovedEvent,
    };
    use crate::domain::{Name, YakId};

    fn metadata() -> EventMetadata {
        let mut metadata = EventMetadata::new(
            Author {
                name: "Matt".to_string(),
                email: "matt@example.com".to_string(),
            },
            Timestamp(1_700_000_000),
        );
        metadata.event_id = Some("evt-1".to_string());
        metadata.commit_sha = Some("abc123".to_string());
        metadata
    }

    fn resolve_name(id: &str) -> String {
        match id {
            "yak-a1b2" => "yak name".to_string(),
            "parent-c3d4" => "parent yak".to_string(),
            "blocker-d5e6" => "blocking yak".to_string(),
            other => other.to_string(),
        }
    }

    #[test]
    fn added_event_serializes_stable_fields_and_payload() {
        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("yak name"),
                id: YakId::from("yak-a1b2"),
                parent_id: Some(YakId::from("parent-c3d4")),
            },
            metadata(),
        );

        let notification = EventNotification::from_event(&event, &resolve_name);

        assert_eq!(notification.event_id, "evt-1");
        assert_eq!(notification.event_type, "Added");
        assert_eq!(notification.yak_id, "yak-a1b2");
        assert_eq!(notification.yak_name, "yak name");
        assert_eq!(notification.timestamp, 1_700_000_000);
        assert_eq!(notification.author, "Matt");
        assert_eq!(
            notification.narrative,
            "Matt added yak name under parent yak"
        );
        assert_eq!(notification.event["name"], "yak name");
        assert_eq!(notification.event["parent_id"], "parent-c3d4");
    }

    #[test]
    fn field_updated_event_serializes_field_name() {
        let event = YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id: YakId::from("yak-a1b2"),
                field_name: ".state".to_string(),
                content: "done".to_string(),
            },
            metadata(),
        );

        let notification = EventNotification::from_event(&event, &resolve_name);

        assert_eq!(notification.event_type, "FieldUpdated");
        assert_eq!(notification.yak_id, "yak-a1b2");
        assert_eq!(notification.narrative, "Matt finished yak name");
        assert_eq!(notification.event["field_name"], ".state");
    }

    #[test]
    fn moved_event_serializes_new_parent() {
        let event = YakEvent::Moved(
            MovedEvent {
                id: YakId::from("yak-a1b2"),
                new_parent: Some(YakId::from("parent-c3d4")),
            },
            metadata(),
        );

        let notification = EventNotification::from_event(&event, &resolve_name);

        assert_eq!(notification.event_type, "Moved");
        assert_eq!(notification.yak_id, "yak-a1b2");
        assert_eq!(notification.event["new_parent"], "parent-c3d4");
    }

    #[test]
    fn removed_event_serializes_removed_id() {
        let event = YakEvent::Removed(
            RemovedEvent {
                id: YakId::from("yak-a1b2"),
            },
            metadata(),
        );

        let notification = EventNotification::from_event(&event, &resolve_name);

        assert_eq!(notification.event_type, "Removed");
        assert_eq!(notification.yak_id, "yak-a1b2");
        assert_eq!(notification.event["id"], "yak-a1b2");
    }

    #[test]
    fn blocker_removed_event_serializes_source() {
        let event = YakEvent::BlockerRemoved(
            BlockerRemovedEvent {
                target: YakId::from("yak-a1b2"),
                source: BlockerSource::Yak(YakId::from("blocker-d5e6")),
            },
            metadata(),
        );

        let notification = EventNotification::from_event(&event, &resolve_name);

        assert_eq!(notification.event_type, "BlockerRemoved");
        assert_eq!(notification.yak_id, "yak-a1b2");
        assert_eq!(
            notification.narrative,
            "Matt removed blocker for yak name by blocking yak"
        );
        assert_eq!(notification.event["source"]["kind"], "yak");
        assert_eq!(notification.event["source"]["id"], "blocker-d5e6");
    }

    #[test]
    fn missing_metadata_uses_consistent_fallbacks() {
        let event = YakEvent::Removed(
            RemovedEvent {
                id: YakId::from("yak-a1b2"),
            },
            EventMetadata::default_legacy(),
        );

        let notification = EventNotification::from_event(&event, &resolve_name);

        assert_eq!(notification.event_id, "-");
        assert_eq!(notification.timestamp, 0);
        assert_eq!(notification.author, "unknown");
    }

    #[test]
    fn json_line_is_compact_single_line_json() {
        let event = YakEvent::Removed(
            RemovedEvent {
                id: YakId::from("yak-a1b2"),
            },
            metadata(),
        );

        let line = event_notification_json_line(&event, &resolve_name).unwrap();
        let parsed: Value = serde_json::from_str(&line).unwrap();

        assert!(!line.contains('\n'));
        assert!(!line.contains("  "));
        assert_eq!(parsed["event_type"], "Removed");
    }
}
