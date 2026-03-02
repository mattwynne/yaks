use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::slug::YakId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedEvent {
    pub id: YakId,
    pub new_parent: Option<YakId>,
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

    fn format_narrative(&self, author: &str, resolve_name: &dyn Fn(&str) -> String) -> String {
        let name = resolve_name(self.id.as_ref());
        match &self.new_parent {
            Some(parent) => {
                let parent_name = resolve_name(parent.as_ref());
                format!("{} moved {} under {}", author, name, parent_name)
            }
            None => format!("{} moved {} to root", author, name),
        }
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(!values.is_empty(), "Moved event requires an id");
        let new_parent = if values.len() >= 2 {
            Some(YakId::from(values[1].clone()))
        } else {
            None
        };
        Ok(Self {
            id: YakId::from(values[0].clone()),
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
            id: YakId::from("child-a1b2"),
            new_parent: Some(YakId::from("new-parent-c3d4")),
        };
        let data = event.format_data();
        let parsed = MovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn roundtrip_to_root() {
        let event = MovedEvent {
            id: YakId::from("child-a1b2"),
            new_parent: None,
        };
        let data = event.format_data();
        let parsed = MovedEvent::parse_data(&data).unwrap();
        assert_eq!(event, parsed);
    }

    #[test]
    fn narrative_under_parent() {
        let event = MovedEvent {
            id: YakId::from("child-a1b2"),
            new_parent: Some(YakId::from("parent-c3d4")),
        };
        assert_eq!(
            event.format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt moved child-a1b2 under parent-c3d4"
        );
    }

    #[test]
    fn narrative_to_root() {
        let event = MovedEvent {
            id: YakId::from("child-a1b2"),
            new_parent: None,
        };
        assert_eq!(
            event.format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt moved child-a1b2 to root"
        );
    }
}
