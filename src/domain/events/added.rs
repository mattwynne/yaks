use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedEvent {
    pub name: String,
    pub id: String,
    pub parent_id: Option<String>,
}

impl EventFormat for AddedEvent {
    fn event_tag(&self) -> &'static str {
        "Added"
    }

    fn format_data(&self) -> String {
        match &self.parent_id {
            Some(parent) => format!("\"{}\" \"{}\" \"{}\"", self.name, self.id, parent),
            None => format!("\"{}\" \"{}\"", self.name, self.id),
        }
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
        let parent_id = if values.len() >= 3 {
            Some(values[2].clone())
        } else {
            None
        };
        Ok(Self {
            name: values[0].clone(),
            id,
            parent_id,
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
            parent_id: None,
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn event_tag() {
        let event = AddedEvent {
            name: "test".to_string(),
            id: "test-x1y2".to_string(),
            parent_id: None,
        };
        assert_eq!(event.event_tag(), "Added");
    }

    #[test]
    fn roundtrip_with_parent_id() {
        let event = AddedEvent {
            name: "child".to_string(),
            id: "child-a1b2".to_string(),
            parent_id: Some("parent-x1y2".to_string()),
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn roundtrip_without_parent_id() {
        let event = AddedEvent {
            name: "root yak".to_string(),
            id: "root-yak-a1b2".to_string(),
            parent_id: None,
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn parse_v2_event_without_id_or_parent() {
        let parsed = AddedEvent::parse_data("\"test yak\"").unwrap();
        assert_eq!(parsed.name, "test yak");
        assert_eq!(parsed.id, "");
        assert_eq!(parsed.parent_id, None);
    }

    #[test]
    fn parse_v3_event_without_parent() {
        let parsed = AddedEvent::parse_data("\"test yak\" \"test-yak-a1b2\"").unwrap();
        assert_eq!(parsed.name, "test yak");
        assert_eq!(parsed.id, "test-yak-a1b2");
        assert_eq!(parsed.parent_id, None);
    }
}
