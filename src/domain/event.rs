// Event domain model - represents a logged yak operation

use anyhow::Result;

use super::event_format::{parse_quoted_values, EventFormat};
use super::event_metadata::EventMetadata;
use super::events::*;
use super::narrative::{highlight, plain, NarrativeSpan};
use super::slug::YakId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YakEvent {
    Added(AddedEvent, EventMetadata),
    Removed(RemovedEvent, EventMetadata),
    Moved(MovedEvent, EventMetadata),
    FieldUpdated(FieldUpdatedEvent, EventMetadata),
    BlockerAdded(BlockerAddedEvent, EventMetadata),
    BlockerUpdated(BlockerUpdatedEvent, EventMetadata),
    BlockerRemoved(BlockerRemovedEvent, EventMetadata),
    ManualBlockerAdded(ManualBlockerAddedEvent, EventMetadata),
    ManualBlockerUpdated(ManualBlockerUpdatedEvent, EventMetadata),
    ManualBlockerRemoved(ManualBlockerRemovedEvent, EventMetadata),
    Compacted(Vec<super::yak::Yak>, Vec<super::slug::YakId>, EventMetadata),
    Migrated(Vec<super::yak::Yak>, Vec<super::slug::YakId>, EventMetadata),
}

impl YakEvent {
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            Self::Added(_, m) => m,
            Self::Removed(_, m) => m,
            Self::Moved(_, m) => m,
            Self::FieldUpdated(_, m) => m,
            Self::BlockerAdded(_, m) => m,
            Self::BlockerUpdated(_, m) => m,
            Self::BlockerRemoved(_, m) => m,
            Self::ManualBlockerAdded(_, m) => m,
            Self::ManualBlockerUpdated(_, m) => m,
            Self::ManualBlockerRemoved(_, m) => m,
            Self::Compacted(_, _, m) => m,
            Self::Migrated(_, _, m) => m,
        }
    }

    pub fn with_metadata(self, metadata: EventMetadata) -> Self {
        match self {
            Self::Added(e, _) => Self::Added(e, metadata),
            Self::Removed(e, _) => Self::Removed(e, metadata),
            Self::Moved(e, _) => Self::Moved(e, metadata),
            Self::FieldUpdated(e, _) => Self::FieldUpdated(e, metadata),
            Self::BlockerAdded(e, _) => Self::BlockerAdded(e, metadata),
            Self::BlockerUpdated(e, _) => Self::BlockerUpdated(e, metadata),
            Self::BlockerRemoved(e, _) => Self::BlockerRemoved(e, metadata),
            Self::ManualBlockerAdded(e, _) => Self::ManualBlockerAdded(e, metadata),
            Self::ManualBlockerUpdated(e, _) => Self::ManualBlockerUpdated(e, metadata),
            Self::ManualBlockerRemoved(e, _) => Self::ManualBlockerRemoved(e, metadata),
            Self::Compacted(s, r, _) => Self::Compacted(s, r, metadata),
            Self::Migrated(s, r, _) => Self::Migrated(s, r, metadata),
        }
    }

    pub fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        match self {
            Self::Added(e, _) => e.format_narrative(author, resolve_name),
            Self::Removed(e, _) => e.format_narrative(author, resolve_name),
            Self::Moved(e, _) => e.format_narrative(author, resolve_name),
            Self::FieldUpdated(e, _) => e.format_narrative(author, resolve_name),
            Self::BlockerAdded(e, _) => e.format_narrative(author, resolve_name),
            Self::BlockerUpdated(e, _) => e.format_narrative(author, resolve_name),
            Self::BlockerRemoved(e, _) => e.format_narrative(author, resolve_name),
            Self::ManualBlockerAdded(e, _) => e.format_narrative(author, resolve_name),
            Self::ManualBlockerUpdated(e, _) => e.format_narrative(author, resolve_name),
            Self::ManualBlockerRemoved(e, _) => e.format_narrative(author, resolve_name),
            Self::Compacted(snapshots, _, _) => {
                let count = snapshots.len();
                vec![
                    highlight(author),
                    plain(&format!(" compacted the event stream ({} yaks)", count)),
                ]
            }
            Self::Migrated(snapshots, _, _) => {
                let count = snapshots.len();
                vec![
                    highlight(author),
                    plain(&format!(" migrated the event stream ({} yaks)", count)),
                ]
            }
        }
    }

    pub fn format_message(&self) -> String {
        match self {
            Self::Added(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Removed(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Moved(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::FieldUpdated(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::BlockerAdded(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::BlockerUpdated(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::BlockerRemoved(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::ManualBlockerAdded(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::ManualBlockerUpdated(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::ManualBlockerRemoved(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Compacted(_, _, _) => "Compacted".to_string(),
            Self::Migrated(_, _, _) => "Migrated".to_string(),
        }
    }

    pub fn parse(message: &str) -> Result<Self> {
        let meta = EventMetadata::default_legacy();
        // Handle dataless events (no ": " separator)
        if message == "Compacted" {
            return Ok(Self::Compacted(vec![], vec![], meta));
        }
        if message == "Migrated" {
            return Ok(Self::Migrated(vec![], vec![], meta));
        }
        let (tag, data) = message
            .split_once(": ")
            .ok_or_else(|| anyhow::anyhow!("Invalid event format: {}", message))?;
        match tag {
            "Added" => Ok(Self::Added(AddedEvent::parse_data(data)?, meta)),
            "Removed" => Ok(Self::Removed(RemovedEvent::parse_data(data)?, meta)),
            "Moved" => Ok(Self::Moved(MovedEvent::parse_data(data)?, meta)),
            "FieldUpdated" => Ok(Self::FieldUpdated(
                FieldUpdatedEvent::parse_data(data)?,
                meta,
            )),
            "BlockerAdded" => Ok(Self::BlockerAdded(
                BlockerAddedEvent::parse_data(data)?,
                meta,
            )),
            "BlockerUpdated" => Ok(Self::BlockerUpdated(
                BlockerUpdatedEvent::parse_data(data)?,
                meta,
            )),
            "BlockerRemoved" => Ok(Self::BlockerRemoved(
                BlockerRemovedEvent::parse_data(data)?,
                meta,
            )),
            "ManualBlockerAdded" => Ok(Self::ManualBlockerAdded(
                ManualBlockerAddedEvent::parse_data(data)?,
                meta,
            )),
            "ManualBlockerUpdated" => Ok(Self::ManualBlockerUpdated(
                ManualBlockerUpdatedEvent::parse_data(data)?,
                meta,
            )),
            "ManualBlockerRemoved" => Ok(Self::ManualBlockerRemoved(
                ManualBlockerRemovedEvent::parse_data(data)?,
                meta,
            )),
            // Backward-compatible parsing of old event formats
            "Renamed" => {
                let values = parse_quoted_values(data)?;
                anyhow::ensure!(values.len() >= 2, "Renamed event requires id and new_name");
                Ok(Self::FieldUpdated(
                    FieldUpdatedEvent {
                        id: YakId::from(values[0].as_str()),
                        field_name: ".name".to_string(),
                        content: values[1].clone(),
                    },
                    meta,
                ))
            }
            "StateUpdated" => {
                let values = parse_quoted_values(data)?;
                anyhow::ensure!(
                    values.len() >= 2,
                    "StateUpdated event requires id and state"
                );
                Ok(Self::FieldUpdated(
                    FieldUpdatedEvent {
                        id: YakId::from(values[0].as_str()),
                        field_name: ".state".to_string(),
                        content: values[1].clone(),
                    },
                    meta,
                ))
            }
            "ContextUpdated" => {
                let values = parse_quoted_values(data)?;
                anyhow::ensure!(!values.is_empty(), "ContextUpdated event requires an id");
                Ok(Self::FieldUpdated(
                    FieldUpdatedEvent {
                        id: YakId::from(values[0].as_str()),
                        field_name: ".context.md".to_string(),
                        content: String::new(),
                    },
                    meta,
                ))
            }
            _ => anyhow::bail!("Unknown event type: {}", tag),
        }
    }

    /// Get the yak ID this event affects (for filtering)
    pub fn yak_id(&self) -> &str {
        match self {
            Self::Added(e, _) => e.id.as_str(),
            Self::Removed(e, _) => e.id.as_str(),
            Self::Moved(e, _) => e.id.as_str(),
            Self::FieldUpdated(e, _) => e.id.as_str(),
            Self::BlockerAdded(e, _) => e.target.as_str(),
            Self::BlockerUpdated(e, _) => e.target.as_str(),
            Self::BlockerRemoved(e, _) => e.target.as_str(),
            Self::ManualBlockerAdded(e, _) => e.target.as_str(),
            Self::ManualBlockerUpdated(e, _) => e.target.as_str(),
            Self::ManualBlockerRemoved(e, _) => e.target.as_str(),
            Self::Compacted(_, _, _) => "",
            Self::Migrated(_, _, _) => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::narrative::to_plain_text;
    use crate::domain::slug::{Name, YakId};

    #[test]
    fn metadata_returns_event_metadata() {
        use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

        let metadata = EventMetadata::new(
            Author {
                name: "Matt".to_string(),
                email: "matt@example.com".to_string(),
            },
            Timestamp(1708300800),
        );
        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from("test-a1b2"),
                parent_id: None,
            },
            metadata.clone(),
        );
        assert_eq!(event.metadata(), &metadata);
    }

    #[test]
    fn format_message_added() {
        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test yak"),
                id: YakId::from("test-yak-a1b2"),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );
        assert_eq!(
            event.format_message(),
            "Added: \"test yak\" \"test-yak-a1b2\""
        );
    }

    #[test]
    fn format_message_field_updated() {
        let event = YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id: YakId::from("test"),
                field_name: ".state".to_string(),
                content: "wip".to_string(),
            },
            EventMetadata::default_legacy(),
        );
        assert_eq!(event.format_message(), "FieldUpdated: \"test\" \".state\"");
    }

    #[test]
    fn parse_roundtrip() {
        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from("test-x1y2"),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );
        let msg = event.format_message();
        let parsed = YakEvent::parse(&msg).unwrap();
        assert_eq!(parsed, event);
    }

    #[test]
    fn parse_unknown_tag_errors() {
        assert!(YakEvent::parse("Unknown: \"foo\"").is_err());
    }

    #[test]
    fn parse_blocker_events() {
        let added = YakEvent::parse(
            "BlockerAdded: \"blocked-yak-a1b2\" \"blocking-yak-c3d4\" \"waiting on API\"",
        )
        .unwrap();
        match added {
            YakEvent::BlockerAdded(event, _) => {
                assert_eq!(event.target, YakId::from("blocked-yak-a1b2"));
                assert_eq!(event.blocker, YakId::from("blocking-yak-c3d4"));
                assert_eq!(event.reason, Some("waiting on API".to_string()));
            }
            _ => panic!("Expected BlockerAdded"),
        }

        let updated = YakEvent::parse(
            "BlockerUpdated: \"blocked-yak-a1b2\" \"blocking-yak-c3d4\" \"new reason\"",
        )
        .unwrap();
        match updated {
            YakEvent::BlockerUpdated(event, _) => {
                assert_eq!(event.target, YakId::from("blocked-yak-a1b2"));
                assert_eq!(event.blocker, YakId::from("blocking-yak-c3d4"));
                assert_eq!(event.reason, Some("new reason".to_string()));
            }
            _ => panic!("Expected BlockerUpdated"),
        }

        let removed =
            YakEvent::parse("BlockerRemoved: \"blocked-yak-a1b2\" \"blocking-yak-c3d4\"").unwrap();
        match removed {
            YakEvent::BlockerRemoved(event, _) => {
                assert_eq!(event.target, YakId::from("blocked-yak-a1b2"));
                assert_eq!(event.blocker, YakId::from("blocking-yak-c3d4"));
            }
            _ => panic!("Expected BlockerRemoved"),
        }

        let manual_added =
            YakEvent::parse("ManualBlockerAdded: \"blocked-yak-a1b2\" \"waiting on vendor\"")
                .unwrap();
        match manual_added {
            YakEvent::ManualBlockerAdded(event, _) => {
                assert_eq!(event.target, YakId::from("blocked-yak-a1b2"));
                assert_eq!(event.reason, "waiting on vendor");
            }
            _ => panic!("Expected ManualBlockerAdded"),
        }

        let manual_updated =
            YakEvent::parse("ManualBlockerUpdated: \"blocked-yak-a1b2\" \"waiting on review\"")
                .unwrap();
        match manual_updated {
            YakEvent::ManualBlockerUpdated(event, _) => {
                assert_eq!(event.target, YakId::from("blocked-yak-a1b2"));
                assert_eq!(event.reason, "waiting on review");
            }
            _ => panic!("Expected ManualBlockerUpdated"),
        }

        let manual_removed = YakEvent::parse("ManualBlockerRemoved: \"blocked-yak-a1b2\"").unwrap();
        match manual_removed {
            YakEvent::ManualBlockerRemoved(event, _) => {
                assert_eq!(event.target, YakId::from("blocked-yak-a1b2"));
            }
            _ => panic!("Expected ManualBlockerRemoved"),
        }
    }

    #[test]
    fn yak_id_returns_correct_id() {
        let event = YakEvent::Moved(
            MovedEvent {
                id: YakId::from("old-a1b2"),
                new_parent: Some(YakId::from("new-parent-c3d4")),
            },
            EventMetadata::default_legacy(),
        );
        assert_eq!(event.yak_id(), "old-a1b2");
    }

    #[test]
    fn parse_legacy_renamed_as_field_updated() {
        let event = YakEvent::parse("Renamed: \"my-yak-a1b2\" \"new name\"").unwrap();
        match event {
            YakEvent::FieldUpdated(e, _) => {
                assert_eq!(e.id, YakId::from("my-yak-a1b2"));
                assert_eq!(e.field_name, ".name");
                assert_eq!(e.content, "new name");
            }
            _ => panic!("Expected FieldUpdated"),
        }
    }

    #[test]
    fn parse_legacy_state_updated_as_field_updated() {
        let event = YakEvent::parse("StateUpdated: \"test-a1b2\" \"wip\"").unwrap();
        match event {
            YakEvent::FieldUpdated(e, _) => {
                assert_eq!(e.id, YakId::from("test-a1b2"));
                assert_eq!(e.field_name, ".state");
                assert_eq!(e.content, "wip");
            }
            _ => panic!("Expected FieldUpdated"),
        }
    }

    #[test]
    fn format_message_compacted() {
        let event = YakEvent::Compacted(vec![], vec![], EventMetadata::default_legacy());
        assert_eq!(event.format_message(), "Compacted");
    }

    #[test]
    fn narrative_compacted_with_count() {
        use crate::domain::event_metadata::{Author, Timestamp};
        use crate::domain::slug::Name;
        use crate::domain::yak::Yak;
        use crate::domain::yak_state::YakState;
        use std::collections::HashMap;
        let author = Author {
            name: "test".to_string(),
            email: "test@test.com".to_string(),
        };
        let snapshots = vec![
            Yak {
                name: Name::from("yak one"),
                id: YakId::from("yak-one-a1b2"),
                state: YakState::Todo,
                context: None,
                parent_id: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: author.clone(),
                created_at: Timestamp(0),
            },
            Yak {
                name: Name::from("yak two"),
                id: YakId::from("yak-two-c3d4"),
                state: YakState::Todo,
                context: None,
                parent_id: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: author,
                created_at: Timestamp(0),
            },
        ];
        let event = YakEvent::Compacted(snapshots, vec![], EventMetadata::default_legacy());
        let spans = event.format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(
            to_plain_text(&spans),
            "Matt compacted the event stream (2 yaks)"
        );
    }

    #[test]
    fn parse_compacted_roundtrip() {
        let event = YakEvent::Compacted(vec![], vec![], EventMetadata::default_legacy());
        let msg = event.format_message();
        let parsed = YakEvent::parse(&msg).unwrap();
        assert_eq!(parsed, event);
    }

    #[test]
    fn compacted_yak_id_is_empty() {
        let event = YakEvent::Compacted(vec![], vec![], EventMetadata::default_legacy());
        assert_eq!(event.yak_id(), "");
    }

    #[test]
    fn parse_legacy_context_updated_as_field_updated() {
        let event = YakEvent::parse("ContextUpdated: \"test-a1b2\"").unwrap();
        match event {
            YakEvent::FieldUpdated(e, _) => {
                assert_eq!(e.id, YakId::from("test-a1b2"));
                assert_eq!(e.field_name, ".context.md");
                assert_eq!(e.content, "");
            }
            _ => panic!("Expected FieldUpdated"),
        }
    }
}
