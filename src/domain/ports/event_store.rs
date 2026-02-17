use crate::domain::{Yak, YakEvent};
use anyhow::Result;

pub trait EventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()>;
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
    fn reset_from_snapshot(&mut self, yaks: &[Yak]) -> Result<usize>;
}

/// Read-only access to the event store
pub trait EventStoreReader {
    fn get_all_events(&self) -> Result<Vec<YakEvent>>;
}
