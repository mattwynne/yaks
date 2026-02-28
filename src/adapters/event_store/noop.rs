// No-op event store - discards all events silently.
//
// Used when YX_SKIP_GIT_CHECKS is set and no git repository is
// available. Allows directory-based storage to function without
// any git infrastructure.

use crate::domain::ports::EventStore;
use crate::domain::YakEvent;
use anyhow::Result;

pub struct NoOpEventStore;

impl EventStore for NoOpEventStore {
    fn append(&mut self, _event: &YakEvent) -> Result<()> {
        Ok(())
    }
    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        Ok(vec![])
    }
    fn compact(&mut self, _metadata: crate::domain::event_metadata::EventMetadata) -> Result<()> {
        Ok(())
    }

    fn wipe(&mut self) -> Result<()> {
        Ok(())
    }

    fn sync(
        &mut self,
        _bus: &mut crate::infrastructure::event_bus::EventBus,
        _output: &dyn crate::domain::ports::DisplayPort,
    ) -> Result<()> {
        anyhow::bail!("Sync not configured")
    }
}
