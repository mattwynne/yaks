use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::slug::{Name, YakId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamedEvent {
    pub id: YakId,
    pub new_name: Name,
}

impl EventFormat for RenamedEvent {
    fn event_tag(&self) -> &'static str {
        "Renamed"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.id, self.new_name)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(values.len() >= 2, "Renamed event requires id and new_name");
        Ok(Self {
            id: YakId::from(values[0].clone()),
            new_name: Name::from(values[1].clone()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let event = RenamedEvent {
            id: YakId::from("my-yak-a1b2"),
            new_name: Name::from("better name"),
        };
        let data = event.format_data();
        let parsed = RenamedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }
}
