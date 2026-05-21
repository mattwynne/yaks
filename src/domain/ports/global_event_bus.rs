use anyhow::Result;
use std::time::Duration;

use crate::domain::YakEvent;

/// Cross-process observation of committed domain events.
///
/// Unlike the local in-process EventBus used for projection dispatch,
/// GlobalEventBus observes the durable event stream from other yx processes.
pub trait GlobalEventBus {
    fn subscribe_from_now(&mut self) -> Result<Box<dyn GlobalEventSubscription + '_>>;
}

pub trait GlobalEventSubscription {
    /// Wait for the next committed batch of events.
    ///
    /// Returns Ok(None) when the optional timeout expires before another batch
    /// is observed. Implementations should only return events committed after
    /// the subscription was created.
    fn next_batch(&mut self, timeout: Option<Duration>) -> Result<Option<Vec<YakEvent>>>;
}
