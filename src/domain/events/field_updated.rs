use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::narrative::{highlight, plain, NarrativeSpan};
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

    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        let name = resolve_name(self.id.as_ref());
        // Strip leading dot from field names (e.g. ".state" → "state")
        let field = self
            .field_name
            .strip_prefix('.')
            .unwrap_or(&self.field_name);
        match field {
            "state" => {
                if self.content == "wip" {
                    vec![highlight(author), plain(" started "), highlight(&name)]
                } else if self.content == "done" {
                    vec![highlight(author), plain(" finished "), highlight(&name)]
                } else if self.content == "todo" {
                    vec![
                        highlight(author),
                        plain(" reset "),
                        highlight(&name),
                        plain(" to todo"),
                    ]
                } else if self.content.is_empty() {
                    // Content not available (read from git)
                    vec![
                        highlight(author),
                        plain(" changed state of "),
                        highlight(&name),
                    ]
                } else {
                    vec![
                        highlight(author),
                        plain(" changed state of "),
                        highlight(&name),
                        plain(&format!(" to {}", self.content)),
                    ]
                }
            }
            "context.md" => vec![
                highlight(author),
                plain(" updated context on "),
                highlight(&name),
            ],
            "tags" => vec![highlight(author), plain(" tagged "), highlight(&name)],
            "name" => vec![highlight(author), plain(" renamed "), highlight(&name)],
            _ => vec![
                highlight(author),
                plain(&format!(" updated {} on ", field)),
                highlight(&name),
            ],
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
    use crate::domain::narrative::to_plain_text;

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
        let spans =
            field_event(".state", "wip").format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt started sync-a1b2");
        assert_eq!(
            spans,
            vec![
                highlight("Matt"),
                plain(" started "),
                highlight("sync-a1b2"),
            ]
        );
    }

    #[test]
    fn narrative_finished() {
        let spans =
            field_event(".state", "done").format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt finished sync-a1b2");
    }

    #[test]
    fn narrative_reset_to_todo() {
        let spans =
            field_event(".state", "todo").format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt reset sync-a1b2 to todo");
    }

    #[test]
    fn narrative_state_no_content() {
        let spans = field_event(".state", "").format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt changed state of sync-a1b2");
    }

    #[test]
    fn narrative_context() {
        let spans = field_event(".context.md", "stuff")
            .format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt updated context on sync-a1b2");
    }

    #[test]
    fn narrative_tags() {
        let spans = field_event(".tags", "").format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt tagged sync-a1b2");
    }

    #[test]
    fn narrative_renamed() {
        let spans = field_event("name", "").format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt renamed sync-a1b2");
    }

    #[test]
    fn narrative_custom_field() {
        let spans = field_event("plan", "").format_narrative("Matt", &|id: &str| id.to_string());
        assert_eq!(to_plain_text(&spans), "Matt updated plan on sync-a1b2");
    }
}
