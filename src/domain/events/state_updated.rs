use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateUpdatedEvent {
    pub id: String,
    pub state: String,
}

impl EventFormat for StateUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "StateUpdated"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.id, self.state)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(
            values.len() >= 2,
            "StateUpdated event requires id and state"
        );
        Ok(Self {
            id: values[0].clone(),
            state: values[1].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = StateUpdatedEvent {
            id: "test-yak-a1b2".to_string(),
            state: "wip".to_string(),
        };
        let data = event.format_data();
        let parsed = StateUpdatedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
