use anyhow::Result;

use crate::domain::event_format::{parse_quoted_values, EventFormat};
use crate::domain::narrative::{highlight, plain, NarrativeSpan};
use crate::domain::slug::YakId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockerSource {
    Yak(YakId),
    Manual,
}

impl BlockerSource {
    pub fn sort_key(&self) -> (u8, &str) {
        match self {
            BlockerSource::Yak(id) => (0, id.as_str()),
            BlockerSource::Manual => (1, ""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocker {
    pub source: BlockerSource,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerAddedEvent {
    pub target: YakId,
    pub blocker: Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerUpdatedEvent {
    pub target: YakId,
    pub blocker: Blocker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerRemovedEvent {
    pub target: YakId,
    pub source: BlockerSource,
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

fn source_name<'a>(source: &'a BlockerSource, resolve_name: &dyn Fn(&str) -> String) -> String {
    match source {
        BlockerSource::Yak(id) => resolve_name(id.as_str()),
        BlockerSource::Manual => "manual blocker".to_string(),
    }
}

fn format_blocker_data(target: &YakId, blocker: &Blocker) -> String {
    match (&blocker.source, normalize_reason(blocker.reason.clone())) {
        (BlockerSource::Yak(id), Some(reason)) => {
            format!("\"{}\" \"yak\" \"{}\" \"{}\"", target, id, reason)
        }
        (BlockerSource::Yak(id), None) => format!("\"{}\" \"yak\" \"{}\"", target, id),
        (BlockerSource::Manual, Some(reason)) => {
            format!("\"{}\" \"manual\" \"{}\"", target, reason)
        }
        (BlockerSource::Manual, None) => format!("\"{}\" \"manual\"", target),
    }
}

fn parse_blocker_data(data: &str) -> Result<(YakId, Blocker)> {
    let values = parse_quoted_values(data)?;
    anyhow::ensure!(
        values.len() >= 2,
        "blocker event requires target and source"
    );
    let target = YakId::from(values[0].as_str());
    match values[1].as_str() {
        "yak" => {
            anyhow::ensure!(values.len() >= 3, "yak blocker event requires blocker id");
            Ok((
                target,
                Blocker {
                    source: BlockerSource::Yak(YakId::from(values[2].as_str())),
                    reason: normalize_reason(values.get(3).cloned()),
                },
            ))
        }
        "manual" => Ok((
            target,
            Blocker {
                source: BlockerSource::Manual,
                reason: normalize_reason(values.get(2).cloned()),
            },
        )),
        _ => Ok((
            target,
            Blocker {
                source: BlockerSource::Yak(YakId::from(values[1].as_str())),
                reason: normalize_reason(values.get(2).cloned()),
            },
        )),
    }
}

fn format_removed_data(target: &YakId, source: &BlockerSource) -> String {
    match source {
        BlockerSource::Yak(id) => format!("\"{}\" \"yak\" \"{}\"", target, id),
        BlockerSource::Manual => format!("\"{}\" \"manual\"", target),
    }
}

fn parse_removed_data(data: &str) -> Result<(YakId, BlockerSource)> {
    let values = parse_quoted_values(data)?;
    anyhow::ensure!(
        values.len() >= 2,
        "BlockerRemoved event requires target and source"
    );
    let target = YakId::from(values[0].as_str());
    match values[1].as_str() {
        "yak" => {
            anyhow::ensure!(
                values.len() >= 3,
                "yak BlockerRemoved event requires blocker id"
            );
            Ok((target, BlockerSource::Yak(YakId::from(values[2].as_str()))))
        }
        "manual" => Ok((target, BlockerSource::Manual)),
        _ => Ok((target, BlockerSource::Yak(YakId::from(values[1].as_str())))),
    }
}

impl EventFormat for BlockerAddedEvent {
    fn event_tag(&self) -> &'static str {
        "BlockerAdded"
    }
    fn format_data(&self) -> String {
        format_blocker_data(&self.target, &self.blocker)
    }
    fn parse_data(data: &str) -> Result<Self> {
        let (target, blocker) = parse_blocker_data(data)?;
        Ok(Self { target, blocker })
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
            highlight(&source_name(&self.blocker.source, resolve_name)),
        ]
    }
}

impl EventFormat for BlockerUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "BlockerUpdated"
    }
    fn format_data(&self) -> String {
        format_blocker_data(&self.target, &self.blocker)
    }
    fn parse_data(data: &str) -> Result<Self> {
        let (target, blocker) = parse_blocker_data(data)?;
        Ok(Self { target, blocker })
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
            highlight(&source_name(&self.blocker.source, resolve_name)),
        ]
    }
}

impl EventFormat for BlockerRemovedEvent {
    fn event_tag(&self) -> &'static str {
        "BlockerRemoved"
    }
    fn format_data(&self) -> String {
        format_removed_data(&self.target, &self.source)
    }
    fn parse_data(data: &str) -> Result<Self> {
        let (target, source) = parse_removed_data(data)?;
        Ok(Self { target, source })
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
            highlight(&source_name(&self.source, resolve_name)),
        ]
    }
}

fn parse_manual_data(data: &str) -> Result<(YakId, String)> {
    let values = parse_quoted_values(data)?;
    anyhow::ensure!(
        values.len() >= 2,
        "manual blocker event requires target and reason"
    );
    Ok((YakId::from(values[0].as_str()), values[1].clone()))
}

impl From<ManualBlockerAddedEvent> for BlockerAddedEvent {
    fn from(e: ManualBlockerAddedEvent) -> Self {
        Self {
            target: e.target,
            blocker: Blocker {
                source: BlockerSource::Manual,
                reason: Some(e.reason),
            },
        }
    }
}
impl From<ManualBlockerUpdatedEvent> for BlockerUpdatedEvent {
    fn from(e: ManualBlockerUpdatedEvent) -> Self {
        Self {
            target: e.target,
            blocker: Blocker {
                source: BlockerSource::Manual,
                reason: Some(e.reason),
            },
        }
    }
}
impl From<ManualBlockerRemovedEvent> for BlockerRemovedEvent {
    fn from(e: ManualBlockerRemovedEvent) -> Self {
        Self {
            target: e.target,
            source: BlockerSource::Manual,
        }
    }
}

impl EventFormat for ManualBlockerAddedEvent {
    fn event_tag(&self) -> &'static str {
        "ManualBlockerAdded"
    }
    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.target, self.reason)
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
        BlockerAddedEvent::from(self.clone()).format_narrative(author, resolve_name)
    }
}
impl EventFormat for ManualBlockerUpdatedEvent {
    fn event_tag(&self) -> &'static str {
        "ManualBlockerUpdated"
    }
    fn format_data(&self) -> String {
        format!("\"{}\" \"{}\"", self.target, self.reason)
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
        BlockerUpdatedEvent::from(self.clone()).format_narrative(author, resolve_name)
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
        BlockerRemovedEvent::from(self.clone()).format_narrative(author, resolve_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_blocker_added_event_with_explicit_source() {
        let event = BlockerAddedEvent {
            target: YakId::from("blocked-yak"),
            blocker: Blocker {
                source: BlockerSource::Yak(YakId::from("blocking-yak")),
                reason: Some("waiting on review".to_string()),
            },
        };
        assert_eq!(event.event_tag(), "BlockerAdded");
        assert_eq!(
            event.format_data(),
            "\"blocked-yak\" \"yak\" \"blocking-yak\" \"waiting on review\""
        );
    }

    #[test]
    fn parses_old_yak_blocker_format() {
        let parsed =
            BlockerAddedEvent::parse_data("\"blocked-yak\" \"blocking-yak\" \"\"").unwrap();
        assert_eq!(
            parsed.blocker.source,
            BlockerSource::Yak(YakId::from("blocking-yak"))
        );
        assert_eq!(parsed.blocker.reason, None);
    }

    #[test]
    fn formats_manual_blocker_as_unified_blocker_event() {
        let event = BlockerAddedEvent {
            target: YakId::from("blocked-yak"),
            blocker: Blocker {
                source: BlockerSource::Manual,
                reason: Some("waiting on vendor".to_string()),
            },
        };
        assert_eq!(
            event.format_data(),
            "\"blocked-yak\" \"manual\" \"waiting on vendor\""
        );
    }

    #[test]
    fn parses_unified_manual_blocker_event() {
        let parsed =
            BlockerUpdatedEvent::parse_data("\"blocked-yak\" \"manual\" \"waiting on vendor\"")
                .unwrap();
        assert_eq!(parsed.blocker.source, BlockerSource::Manual);
        assert_eq!(parsed.blocker.reason, Some("waiting on vendor".to_string()));
    }
}
