use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedEvent {
    pub name: String,
}

impl EventFormat for RemovedEvent {
    fn event_tag(&self) -> &'static str {
        "Removed"
    }

    fn format_data(&self) -> String {
        format!("\"{}\"", self.name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Removed event requires a name");
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
        let event = RemovedEvent {
            name: "test yak".to_string(),
        };
        let data = event.format_data();
        let parsed = RemovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
