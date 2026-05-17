use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::narrative::{highlight, plain, NarrativeSpan};
use crate::domain::slug::YakId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerAddedEvent {
    pub target: YakId,
    pub blocker: YakId,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerUpdatedEvent {
    pub target: YakId,
    pub blocker: YakId,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerRemovedEvent {
    pub target: YakId,
    pub blocker: YakId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualBlockerAddedEvent {
    pub target: YakId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualBlockerUpdatedEvent {
    pub target: YakId,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualBlockerRemovedEvent {
    pub target: YakId,
}

pub(crate) fn normalize_reason(reason: Option<String>) -> Option<String> {
    reason.filter(|reason| !reason.is_empty())
}

fn format_reason_data(target: &YakId, blocker: &YakId, reason: &Option<String>) -> String {
    match normalize_reason(reason.clone()) {
        Some(reason) => format!("\"{}\" \"{}\" \"{}\"", target, blocker, reason),
        None => format!("\"{}\" \"{}\"", target, blocker),
    }
}

fn parse_reason_data(data: &str) -> Result<(YakId, YakId, Option<String>)> {
    let values = parse_quoted_values(data)?;
    anyhow::ensure!(
        values.len() >= 2,
        "blocker event requires target and blocker"
    );
    Ok((
        YakId::from(values[0].as_str()),
        YakId::from(values[1].as_str()),
        normalize_reason(values.get(2).cloned()),
    ))
}

impl EventFormat for BlockerAddedEvent {
    fn event_tag(&self) -> &'static str {
        "BlockerAdded"
    }

    fn format_data(&self) -> String {
        format_reason_data(&self.target, &self.blocker, &self.reason)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let (target, blocker, reason) = parse_reason_data(data)?;
        Ok(Self {
            target,
            blocker,
            reason,
        })
    }

    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        vec![
            highlight(author),
            plain(" marked "),
            highlight(&resolve_name(self.target.as_str())),
            plain(" blocked by "),
            highlight(&resolve_name(self.blocker.as_str())),
        ]
    }
}

impl EventFormat for BlockerUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "BlockerUpdated"
    }

    fn format_data(&self) -> String {
        format_reason_data(&self.target, &self.blocker, &self.reason)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let (target, blocker, reason) = parse_reason_data(data)?;
        Ok(Self {
            target,
            blocker,
            reason,
        })
    }

    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        vec![
            highlight(author),
            plain(" updated blocker for "),
            highlight(&resolve_name(self.target.as_str())),
            plain(" by "),
            highlight(&resolve_name(self.blocker.as_str())),
        ]
    }
}

fn format_manual_data(target: &YakId, reason: &str) -> String {
    format!("\"{}\" \"{}\"", target, reason)
}

fn parse_manual_data(data: &str) -> Result<(YakId, String)> {
    let values = parse_quoted_values(data)?;
    anyhow::ensure!(
        values.len() >= 2,
        "manual blocker event requires target and reason"
    );
    Ok((YakId::from(values[0].as_str()), values[1].clone()))
}

impl EventFormat for BlockerRemovedEvent {
    fn event_tag(&self) -> &'static str {
        "BlockerRemoved"
    }

    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.target, self.blocker)
    }

    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(
            values.len() >= 2,
            "BlockerRemoved event requires target and blocker"
        );
        Ok(Self {
            target: YakId::from(values[0].as_str()),
            blocker: YakId::from(values[1].as_str()),
        })
    }

    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        vec![
            highlight(author),
            plain(" removed blocker for "),
            highlight(&resolve_name(self.target.as_str())),
            plain(" by "),
            highlight(&resolve_name(self.blocker.as_str())),
        ]
    }
}

impl EventFormat for ManualBlockerAddedEvent {
    fn event_tag(&self) -> &'static str {
        "ManualBlockerAdded"
    }
    fn format_data(&self) -> String {
        format_manual_data(&self.target, &self.reason)
    }
    fn parse_data(data: &str) -> Result<Self> {
        let (target, reason) = parse_manual_data(data)?;
        Ok(Self { target, reason })
    }
    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        vec![
            highlight(author),
            plain(" marked "),
            highlight(&resolve_name(self.target.as_str())),
            plain(" manually blocked"),
        ]
    }
}

impl EventFormat for ManualBlockerUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "ManualBlockerUpdated"
    }
    fn format_data(&self) -> String {
        format_manual_data(&self.target, &self.reason)
    }
    fn parse_data(data: &str) -> Result<Self> {
        let (target, reason) = parse_manual_data(data)?;
        Ok(Self { target, reason })
    }
    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        vec![
            highlight(author),
            plain(" updated manual blocker for "),
            highlight(&resolve_name(self.target.as_str())),
        ]
    }
}

impl EventFormat for ManualBlockerRemovedEvent {
    fn event_tag(&self) -> &'static str {
        "ManualBlockerRemoved"
    }
    fn format_data(&self) -> String {
        format!("\"{}\"", self.target)
    }
    fn parse_data(data: &str) -> Result<Self> {
        let values = parse_quoted_values(data)?;
        anyhow::ensure!(
            !values.is_empty(),
            "ManualBlockerRemoved event requires target"
        );
        Ok(Self {
            target: YakId::from(values[0].as_str()),
        })
    }
    fn format_narrative(
        &self,
        author: &str,
        resolve_name: &dyn Fn(&str) -> String,
    ) -> Vec<NarrativeSpan> {
        vec![
            highlight(author),
            plain(" removed manual blocker for "),
            highlight(&resolve_name(self.target.as_str())),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_blocker_added_event_with_reason() {
        let event = BlockerAddedEvent {
            target: YakId::from("blocked-yak"),
            blocker: YakId::from("blocking-yak"),
            reason: Some("waiting on review".to_string()),
        };

        assert_eq!(event.event_tag(), "BlockerAdded");
        assert_eq!(
            event.format_data(),
            "\"blocked-yak\" \"blocking-yak\" \"waiting on review\""
        );
    }

    #[test]
    fn formats_blocker_added_event_with_empty_reason_without_reason_value() {
        let event = BlockerAddedEvent {
            target: YakId::from("blocked-yak"),
            blocker: YakId::from("blocking-yak"),
            reason: Some(String::new()),
        };

        assert_eq!(event.event_tag(), "BlockerAdded");
        assert_eq!(event.format_data(), "\"blocked-yak\" \"blocking-yak\"");
    }

    #[test]
    fn formats_blocker_updated_event_with_no_reason() {
        let event = BlockerUpdatedEvent {
            target: YakId::from("blocked-yak"),
            blocker: YakId::from("blocking-yak"),
            reason: None,
        };

        assert_eq!(event.event_tag(), "BlockerUpdated");
        assert_eq!(event.format_data(), "\"blocked-yak\" \"blocking-yak\"");
    }

    #[test]
    fn formats_blocker_removed_event() {
        let event = BlockerRemovedEvent {
            target: YakId::from("blocked-yak"),
            blocker: YakId::from("blocking-yak"),
        };

        assert_eq!(event.event_tag(), "BlockerRemoved");
        assert_eq!(event.format_data(), "\"blocked-yak\" \"blocking-yak\"");
    }

    #[test]
    fn formats_manual_blocker_events() {
        let added = ManualBlockerAddedEvent {
            target: YakId::from("blocked-yak"),
            reason: "waiting on vendor".to_string(),
        };
        assert_eq!(added.event_tag(), "ManualBlockerAdded");
        assert_eq!(added.format_data(), "\"blocked-yak\" \"waiting on vendor\"");

        let updated = ManualBlockerUpdatedEvent {
            target: YakId::from("blocked-yak"),
            reason: "waiting on review".to_string(),
        };
        assert_eq!(updated.event_tag(), "ManualBlockerUpdated");
        assert_eq!(
            updated.format_data(),
            "\"blocked-yak\" \"waiting on review\""
        );

        let removed = ManualBlockerRemovedEvent {
            target: YakId::from("blocked-yak"),
        };
        assert_eq!(removed.event_tag(), "ManualBlockerRemoved");
        assert_eq!(removed.format_data(), "\"blocked-yak\"");
    }

    #[test]
    fn parses_empty_added_reason_as_none() {
        let parsed =
            BlockerAddedEvent::parse_data("\"blocked-yak\" \"blocking-yak\" \"\"").unwrap();

        assert_eq!(parsed.reason, None);
    }

    #[test]
    fn parses_empty_updated_reason_as_none() {
        let parsed =
            BlockerUpdatedEvent::parse_data("\"blocked-yak\" \"blocking-yak\" \"\"").unwrap();

        assert_eq!(parsed.reason, None);
    }
}
