use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedEvent {
    pub name: String,
}

impl EventFormat for AddedEvent {
    fn event_tag(&self) -> &'static str {
        "Added"
    }

    fn format_data(&self) -> String {
        format!("\"{}\"", self.name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Added event requires a name");
        Ok(Self {
            name: values[0].clone(),
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
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn event_tag() {
        let event = AddedEvent {
            name: "test".to_string(),
        };
        assert_eq!(event.event_tag(), "Added");
    }
}
