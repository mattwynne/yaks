// Event domain model - represents a logged yak operation

use anyhow::Result;

use super::event_format::EventFormat;
use super::events::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YakEvent {
    Added(AddedEvent),
    Removed(RemovedEvent),
    Moved(MovedEvent),
    Renamed(RenamedEvent),
    ContextUpdated(ContextUpdatedEvent),
    StateUpdated(StateUpdatedEvent),
    FieldUpdated(FieldUpdatedEvent),
}

impl YakEvent {
    pub fn format_message(&self) -> String {
        match self {
            Self::Added(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Removed(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Moved(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::Renamed(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::ContextUpdated(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::StateUpdated(e) => format!("{}: {}", e.event_tag(), e.format_data()),
            Self::FieldUpdated(e) => format!("{}: {}", e.event_tag(), e.format_data()),
        }
    }

    pub fn parse(message: &str) -> Result<Self> {
        let (tag, data) = message
            .split_once(": ")
            .ok_or_else(|| anyhow::anyhow!("Invalid event format: {}", message))?;
        match tag {
            "Added" => Ok(Self::Added(AddedEvent::parse_data(data)?)),
            "Removed" => Ok(Self::Removed(RemovedEvent::parse_data(data)?)),
            "Moved" => Ok(Self::Moved(MovedEvent::parse_data(data)?)),
            "Renamed" => Ok(Self::Renamed(RenamedEvent::parse_data(data)?)),
            "ContextUpdated" => Ok(Self::ContextUpdated(ContextUpdatedEvent::parse_data(data)?)),
            "StateUpdated" => Ok(Self::StateUpdated(StateUpdatedEvent::parse_data(data)?)),
            "FieldUpdated" => Ok(Self::FieldUpdated(FieldUpdatedEvent::parse_data(data)?)),
            _ => anyhow::bail!("Unknown event type: {}", tag),
        }
    }

    /// Get the yak name this event affects (for filtering)
    #[cfg(any(test, feature = "test-support"))]
    pub fn yak_name(&self) -> &str {
        match self {
            Self::Added(e) => &e.name,
            Self::Removed(e) => &e.id,
            Self::Moved(e) => &e.id,
            Self::Renamed(e) => &e.id,
            Self::ContextUpdated(e) => &e.id,
            Self::StateUpdated(e) => &e.id,
            Self::FieldUpdated(e) => &e.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_message_added() {
        let event = YakEvent::Added(AddedEvent {
            name: "test yak".to_string(),
            id: "test-yak-a1b2".to_string(),
            parent_id: None,
        });
        assert_eq!(
            event.format_message(),
            "Added: \"test yak\" \"test-yak-a1b2\""
        );
    }

    #[test]
    fn format_message_state_updated() {
        let event = YakEvent::StateUpdated(StateUpdatedEvent {
            id: "test".to_string(),
            state: "wip".to_string(),
        });
        assert_eq!(event.format_message(), "StateUpdated: \"test\" \"wip\"");
    }

    #[test]
    fn parse_roundtrip() {
        let event = YakEvent::Added(AddedEvent {
            name: "test".to_string(),
            id: "test-x1y2".to_string(),
            parent_id: None,
        });
        let msg = event.format_message();
        let parsed = YakEvent::parse(&msg).unwrap();
        assert_eq!(parsed, event);
    }

    #[test]
    fn parse_unknown_tag_errors() {
        assert!(YakEvent::parse("Unknown: \"foo\"").is_err());
    }

    #[test]
    fn yak_name_returns_correct_name() {
        let event = YakEvent::Moved(MovedEvent {
            id: "old-a1b2".to_string(),
            new_parent: Some("new-parent-c3d4".to_string()),
        });
        assert_eq!(event.yak_name(), "old-a1b2");
    }
}
