use crate::domain::{Yak, YakEvent};
use crate::infrastructure::event_bus::EventBus;
use anyhow::Result;

use super::DisplayPort;

pub trait EventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()>;
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
    fn reset_from_snapshot(&mut self, yaks: &[Yak]) -> Result<usize>;
    fn sync(&mut self, bus: &mut EventBus, output: &dyn DisplayPort) -> Result<()>;

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
