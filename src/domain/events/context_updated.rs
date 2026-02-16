use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::slug::YakId;

/// Note: `content` is NOT serialized in the commit message because it
/// is stored in the git tree (context.md blob). When reading events
/// back from git, `content` will be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextUpdatedEvent {
    pub id: YakId,
    pub content: String,
}

impl EventFormat for ContextUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "ContextUpdated"
    }

    fn format_data(&self) -> String {
        format!("\"{}\"", self.id)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "ContextUpdated event requires an id");
        Ok(Self {
            id: YakId::from(values[0].clone()),
            content: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_excludes_content() {
        let event = ContextUpdatedEvent {
            id: YakId::from("test-yak-a1b2"),
            content: "some long context".to_string(),
        };
        assert_eq!(event.format_data(), "\"test-yak-a1b2\"");
    }

    #[test]
    fn parse_sets_empty_content() {
        let parsed = ContextUpdatedEvent::parse_data("\"test-yak-a1b2\"").unwrap();
        assert_eq!(parsed.id, YakId::from("test-yak-a1b2"));
        assert_eq!(parsed.content, "");
    }
}
