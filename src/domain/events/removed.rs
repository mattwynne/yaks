use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::narrative::{highlight, plain, NarrativeSpan};
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

    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        let name = resolve_name(self.id.as_ref());
        vec![highlight(author), plain(" removed "), highlight(&name)]
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
    use crate::domain::narrative::to_plain_text;

    #[test]
    fn roundtrip() {
        let event = RemovedEvent {
            id: YakId::from("test-yak-a1b2"),
        };
        let data = event.format_data();
        let parsed = RemovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn narrative() {
        let event = RemovedEvent {
            id: YakId::from("old-yak-a1b2"),
        };
        let spans = event.format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt removed old-yak-a1b2");
        assert_eq!(
            spans,
            vec![
                highlight("Matt"),
                plain(" removed "),
                highlight("old-yak-a1b2"),
            ]
        );
    }
}
