use std::collections::HashMap;

use super::event_metadata::{Author, Timestamp};
use super::slug::{Name, YakId};

/// A point-in-time snapshot of a yak's full state.
/// Used inside `Compacted` events to carry the complete state
/// without synthesizing fake Added/FieldUpdated events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YakSnapshot {
    pub id: YakId,
    pub name: Name,
    pub parent_id: Option<YakId>,
    pub state: String,
    pub context: Option<String>,
    pub fields: HashMap<String, String>,
    pub created_by: Author,
    pub created_at: Timestamp,
}

impl From<&super::yak::YakView> for YakSnapshot {
    fn from(yak: &super::yak::YakView) -> Self {
        Self {
            id: yak.id.clone(),
            name: yak.name.clone(),
            parent_id: yak.parent_id.clone(),
            state: yak.state.clone(),
            context: yak.context.clone(),
            fields: yak.fields.clone(),
            created_by: yak.created_by.clone(),
            created_at: yak.created_at,
        }
    }
}
