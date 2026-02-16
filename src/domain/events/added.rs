use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedEvent {
    pub name: String,
    pub id: String,
}

impl EventFormat for AddedEvent {
    fn event_tag(&self) -> &'static str {
        "Added"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.name, self.id)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Added event requires a name");
        let id = if values.len() >= 2 {
            values[1].clone()
        } else {
            // Backward compat: v2 events have no id
            String::new()
        };
        Ok(Self {
            name: values[0].clone(),
            id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = AddedEvent {
            name: "test yak".to_string(),
            id: "test-yak-a1b2".to_string(),
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn parse_v2_event_without_id() {
        let parsed = AddedEvent::parse_data("\"test yak\"").unwrap();
        assert_eq!(parsed.name, "test yak");
        assert_eq!(parsed.id, "");
    }

    #[test]
    fn event_tag() {
        let event = AddedEvent {
            name: "test".to_string(),
            id: "test-x1y2".to_string(),
        };
        assert_eq!(event.event_tag(), "Added");
    }
}
