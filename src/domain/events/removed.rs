use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::slug::YakId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedEvent {
    pub id: YakId,
}

impl EventFormat for RemovedEvent {
    fn event_tag(&self) -> &'static str {
        "Removed"
    }

    fn format_data(&self) -> String {
        format!("\"{}\"", self.id)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Removed event requires an id");
        Ok(Self {
            id: YakId::from(values[0].clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = RemovedEvent {
            id: YakId::from("test-yak-a1b2"),
        };
        let data = event.format_data();
        let parsed = RemovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
