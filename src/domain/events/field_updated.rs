use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::slug::YakId;

/// Note: `content` is NOT serialized in the commit message because it
/// is stored in the git tree (as a blob). When reading events back
/// from git, `content` will be empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldUpdatedEvent {
    pub id: YakId,
    pub field_name: String,
    pub content: String,
}

impl EventFormat for FieldUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "FieldUpdated"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.id, self.field_name)
    }

    fn format_narrative(&self, author: &str, resolve_name: &dyn Fn(&str) -> String) -> String {
        let name = resolve_name(self.id.as_ref());
        // Strip leading dot from field names (e.g. ".state" → "state")
        let field = self
            .field_name
            .strip_prefix('.')
            .unwrap_or(&self.field_name);
        match field {
            "state" => {
                if self.content == "wip" {
                    format!("{} started {}", author, name)
                } else if self.content == "done" {
                    format!("{} finished {}", author, name)
                } else if self.content == "todo" {
                    format!("{} reset {} to todo", author, name)
                } else if self.content.is_empty() {
                    // Content not available (read from git)
                    format!("{} changed state of {}", author, name)
                } else {
                    format!("{} changed state of {} to {}", author, name, self.content)
                }
            }
            "context.md" => format!("{} updated context on {}", author, name),
            "tags" => format!("{} tagged {}", author, name),
            "name" => format!("{} renamed {}", author, name),
            _ => format!("{} updated {} on {}", author, field, name),
        }
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(
            values.len() >= 2,
            "FieldUpdated event requires id and field_name"
        );
        Ok(Self {
            id: YakId::from(values[0].clone()),
            field_name: values[1].clone(),
            content: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_excludes_content() {
        let event = FieldUpdatedEvent {
            id: YakId::from("test-yak-a1b2"),
            field_name: "notes".to_string(),
            content: "stuff".to_string(),
        };
        assert_eq!(event.format_data(), "\"test-yak-a1b2\" \"notes\"");
    }

    #[test]
    fn parse_sets_empty_content() {
        let parsed = FieldUpdatedEvent::parse_data("\"test-yak-a1b2\" \"notes\"").unwrap();
        assert_eq!(parsed.id, YakId::from("test-yak-a1b2"));
        assert_eq!(parsed.field_name, "notes");
        assert_eq!(parsed.content, "");
    }

    fn field_event(field_name: &str, content: &str) -> FieldUpdatedEvent {
        FieldUpdatedEvent {
            id: YakId::from("sync-a1b2"),
            field_name: field_name.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn narrative_started() {
        assert_eq!(
            field_event(".state", "wip").format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt started sync-a1b2"
        );
    }

    #[test]
    fn narrative_finished() {
        assert_eq!(
            field_event(".state", "done").format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt finished sync-a1b2"
        );
    }

    #[test]
    fn narrative_reset_to_todo() {
        assert_eq!(
            field_event(".state", "todo").format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt reset sync-a1b2 to todo"
        );
    }

    #[test]
    fn narrative_state_no_content() {
        assert_eq!(
            field_event(".state", "").format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt changed state of sync-a1b2"
        );
    }

    #[test]
    fn narrative_context() {
        assert_eq!(
            field_event(".context.md", "stuff")
                .format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt updated context on sync-a1b2"
        );
    }

    #[test]
    fn narrative_tags() {
        assert_eq!(
            field_event(".tags", "").format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt tagged sync-a1b2"
        );
    }

    #[test]
    fn narrative_renamed() {
        assert_eq!(
            field_event("name", "").format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt renamed sync-a1b2"
        );
    }

    #[test]
    fn narrative_custom_field() {
        assert_eq!(
            field_event("plan", "").format_narrative("Matt", &|id: &str| id.to_string()),
            "Matt updated plan on sync-a1b2"
        );
    }
}
