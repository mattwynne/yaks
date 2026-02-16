use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::slug::YakId;

/// Note: `content` is NOT serialized in the commit message because it
/// is stored in the git tree (as a blob). When reading events back
/// from git, `content` will be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldUpdatedEvent {
    pub id: YakId,
    pub field_name: String,
    pub content: String,
}

impl EventFormat for FieldUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "FieldUpdated"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.id, self.field_name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(
            values.len() >= 2,
            "FieldUpdated event requires id and field_name"
        );
        Ok(Self {
            id: YakId::from(values[0].clone()),
            field_name: values[1].clone(),
            content: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_excludes_content() {
        let event = FieldUpdatedEvent {
            id: YakId::from("test-yak-a1b2"),
            field_name: "notes".to_string(),
            content: "stuff".to_string(),
        };
        assert_eq!(event.format_data(), "\"test-yak-a1b2\" \"notes\"");
    }

    #[test]
    fn parse_sets_empty_content() {
        let parsed = FieldUpdatedEvent::parse_data("\"test-yak-a1b2\" \"notes\"").unwrap();
        assert_eq!(parsed.id, YakId::from("test-yak-a1b2"));
        assert_eq!(parsed.field_name, "notes");
        assert_eq!(parsed.content, "");
    }
}
