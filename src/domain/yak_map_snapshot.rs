use crate::domain::slug::YakId;
use crate::domain::Yak;

/// Complete snapshot of the YakMap aggregate state.
///
/// `Yak` records are only one part of the aggregate. Aggregate-level blocker
/// relationships live on `YakMap`, so compaction must persist them explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct YakMapSnapshot {
    pub yaks: Vec<Yak>,
    pub removed_yak_ids: Vec<YakId>,
    pub blockers: Vec<YakBlockerSnapshot>,
    pub manual_blockers: Vec<ManualBlockerSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YakBlockerSnapshot {
    pub target: YakId,
    pub blocker: YakId,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualBlockerSnapshot {
    pub target: YakId,
    pub reason: String,
}

impl YakMapSnapshot {
    pub fn legacy(yaks: Vec<Yak>, removed_yak_ids: Vec<YakId>) -> Self {
        Self {
            yaks,
            removed_yak_ids,
            blockers: Vec::new(),
            manual_blockers: Vec::new(),
        }
    }

    pub fn yak_count(&self) -> usize {
        self.yaks.len()
    }
}
