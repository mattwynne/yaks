use crate::domain::event_metadata::EventMetadata;
use crate::domain::YakEvent;
use crate::infrastructure::event_bus::EventBus;
use anyhow::Result;

use super::DisplayPort;

pub trait EventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()>;
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
    fn sync(&mut self, bus: &mut EventBus, output: &dyn DisplayPort) -> Result<()>;

    /// Create and append a Compacted event, which represents a
    /// snapshot of the full state at this point in the event stream.
    fn compact(&mut self, metadata: EventMetadata) -> Result<()>;

    /// Clear all events, preparing for a full replay.
    /// Used by the reset-git-from-disk workflow.
    fn wipe(&mut self) -> Result<()>;

    fn get_events(&self, yak_id: &str) -> Result<Vec<YakEvent>> {
        Ok(self
            .get_all_events()?
            .into_iter()
            .filter(|e| e.yak_id() == yak_id)
            .collect())
    }
}

/// Read-only access to the event store
pub trait EventStoreReader {
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
}
