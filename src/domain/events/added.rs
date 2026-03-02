use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::narrative::{highlight, plain, NarrativeSpan};
use crate::domain::slug::{Name, YakId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedEvent {
    pub name: Name,
    pub id: YakId,
    pub parent_id: Option<YakId>,
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

    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        match &self.parent_id {
            Some(parent) => {
                let parent_name = resolve_name(parent.as_ref());
                vec![
                    highlight(author),
                    plain(" added "),
                    highlight(self.name.as_ref()),
                    plain(" under "),
                    highlight(&parent_name),
                ]
            }
            None => vec![
                highlight(author),
                plain(" added "),
                highlight(self.name.as_ref()),
            ],
        }
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Added event requires a name");
        let id = if values.len() >= 2 {
            YakId::from(values[1].clone())
        } else {
            // Backward compat: v2 events have no id
            YakId::from("")
        };
        let parent_id = if values.len() >= 3 {
            Some(YakId::from(values[2].clone()))
        } else {
            None
        };
        Ok(Self {
            name: Name::from(values[0].clone()),
            id,
            parent_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::narrative::to_plain_text;

    #[test]
    fn roundtrip() {
        let event = AddedEvent {
            name: Name::from("test yak"),
            id: YakId::from("test-yak-a1b2"),
            parent_id: None,
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn event_tag() {
        let event = AddedEvent {
            name: Name::from("test"),
            id: YakId::from("test-x1y2"),
            parent_id: None,
        };
        assert_eq!(event.event_tag(), "Added");
    }

    #[test]
    fn roundtrip_with_parent_id() {
        let event = AddedEvent {
            name: Name::from("child"),
            id: YakId::from("child-a1b2"),
            parent_id: Some(YakId::from("parent-x1y2")),
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn roundtrip_without_parent_id() {
        let event = AddedEvent {
            name: Name::from("root yak"),
            id: YakId::from("root-yak-a1b2"),
            parent_id: None,
        };
        let data = event.format_data();
        let parsed = AddedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn parse_v2_event_without_id_or_parent() {
        let parsed = AddedEvent::parse_data("\"test yak\"").unwrap();
        assert_eq!(parsed.name, Name::from("test yak"));
        assert_eq!(parsed.id, YakId::from(""));
        assert_eq!(parsed.parent_id, None);
    }

    #[test]
    fn parse_v3_event_without_parent() {
        let parsed = AddedEvent::parse_data("\"test yak\" \"test-yak-a1b2\"").unwrap();
        assert_eq!(parsed.name, Name::from("test yak"));
        assert_eq!(parsed.id, YakId::from("test-yak-a1b2"));
        assert_eq!(parsed.parent_id, None);
    }

    #[test]
    fn narrative_root() {
        let event = AddedEvent {
            name: Name::from("Fix the Bug"),
            id: YakId::from("fix-the-bug-a1b2"),
            parent_id: None,
        };
        let spans = event.format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt added Fix the Bug");
        assert_eq!(
            spans,
            vec![
                highlight("Matt"),
                plain(" added "),
                highlight("Fix the Bug"),
            ]
        );
    }

    #[test]
    fn narrative_nested() {
        let event = AddedEvent {
            name: Name::from("sync"),
            id: YakId::from("sync-a1b2"),
            parent_id: Some(YakId::from("things-c3d4")),
        };
        let spans = event.format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt added sync under things-c3d4");
        assert_eq!(
            spans,
            vec![
                highlight("Matt"),
                plain(" added "),
                highlight("sync"),
                plain(" under "),
                highlight("things-c3d4"),
            ]
        );
    }
}
