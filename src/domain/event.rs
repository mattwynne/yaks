// Event domain model - represents a logged yak operation

use anyhow::Result;

use super::event_format::{parse_quoted_values, EventFormat};
use super::event_metadata::EventMetadata;
use super::events::*;
use super::slug::YakId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YakEvent {
    Added(AddedEvent, EventMetadata),
    Removed(RemovedEvent, EventMetadata),
    Moved(MovedEvent, EventMetadata),
    FieldUpdated(FieldUpdatedEvent, EventMetadata),
    Compacted(Vec<super::yak_snapshot::YakSnapshot>, EventMetadata),
}

impl YakEvent {
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            Self::Added(_, m) => m,
            Self::Removed(_, m) => m,
            Self::Moved(_, m) => m,
            Self::FieldUpdated(_, m) => m,
            Self::Compacted(_, m) => m,
        }
    }

    pub fn with_metadata(self, metadata: EventMetadata) -> Self {
        match self {
            Self::Added(e, _) => Self::Added(e, metadata),
            Self::Removed(e, _) => Self::Removed(e, metadata),
            Self::Moved(e, _) => Self::Moved(e, metadata),
            Self::FieldUpdated(e, _) => Self::FieldUpdated(e, metadata),
            Self::Compacted(s, _) => Self::Compacted(s, metadata),
        }
    }

    pub fn format_message(&self) -> String {
        match self {
            Self::Added(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Removed(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Moved(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::FieldUpdated(e, _) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Compacted(_, _) => "Compacted".to_string(),
        }
    }

    pub fn parse(message: &str) -> Result<Self> {
        let meta = EventMetadata::default_legacy();
        // Handle dataless events (no ": " separator)
        if message == "Compacted" {
            return Ok(Self::Compacted(vec![], meta));
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
            Self::Compacted(_, _) => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let event = YakEvent::Compacted(vec![], EventMetadata::default_legacy());
        assert_eq!(event.format_message(), "Compacted");
    }

    #[test]
    fn parse_compacted_roundtrip() {
        let event = YakEvent::Compacted(vec![], EventMetadata::default_legacy());
        let msg = event.format_message();
        let parsed = YakEvent::parse(&msg).unwrap();
        assert_eq!(parsed, event);
    }

    #[test]
    fn compacted_yak_id_is_empty() {
        let event = YakEvent::Compacted(vec![], EventMetadata::default_legacy());
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
