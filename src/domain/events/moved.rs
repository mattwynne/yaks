use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedEvent {
    pub id: String,
    pub new_parent: Option<String>,
}

impl EventFormat for MovedEvent {
    fn event_tag(&self) -> &'static str {
        "Moved"
    }

    fn format_data(&self) -> String {
        match &self.new_parent {
            Some(parent) => format!("\"{}\" \"{}\"", self.id, parent),
            None => format!("\"{}\"", self.id),
        }
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Moved event requires an id");
        let new_parent = if values.len() >= 2 {
            Some(values[1].clone())
        } else {
            None
        };
        Ok(Self {
            id: values[0].clone(),
            new_parent,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_with_parent() {
        let event = MovedEvent {
            id: "child-a1b2".to_string(),
            new_parent: Some("new-parent-c3d4".to_string()),
        };
        let data = event.format_data();
        let parsed = MovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn roundtrip_to_root() {
        let event = MovedEvent {
            id: "child-a1b2".to_string(),
            new_parent: None,
        };
        let data = event.format_data();
        let parsed = MovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
