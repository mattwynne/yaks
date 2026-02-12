use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedEvent {
    pub old_name: String,
    pub new_name: String,
}

impl EventFormat for MovedEvent {
    fn event_tag(&self) -> &'static str {
        "Moved"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.old_name, self.new_name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(values.len() >= 2, "Moved event requires old and new names");
        Ok(Self {
            old_name: values[0].clone(),
            new_name: values[1].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = MovedEvent {
            old_name: "old name".to_string(),
            new_name: "new name".to_string(),
        };
        let data = event.format_data();
        let parsed = MovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
