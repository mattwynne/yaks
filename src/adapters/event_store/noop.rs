// No-op event store - discards all events silently.
//
// Used when YX_SKIP_GIT_CHECKS is set and no git repository is
// available. Allows directory-based storage to function without
// any git infrastructure.

use crate::domain::ports::EventStore;
use crate::domain::{Yak, YakEvent};
use anyhow::Result;

pub struct NoOpEventStore;

impl EventStore for NoOpEventStore {
    fn append(&mut self, _event: &YakEvent) -> Result<()> {
        Ok(())
    }
    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        Ok(vec![])
    }
    fn reset_from_snapshot(&mut self, _yaks: &[Yak]) -> Result<usize> {
        Ok(0)
    }

    fn sync(
        &mut self,
        _peer: &mut dyn EventStore,
        _bus: &mut crate::infrastructure::event_bus::EventBus,
        _output: &dyn crate::domain::ports::DisplayPort,
    ) -> Result<()> {
        todo!("sync not yet implemented")
    }
}
