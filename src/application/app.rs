// Application struct - bundles infrastructure adapters for use case execution

use crate::ports::{DisplayPort, LogPort, StoragePort};

/// Application bundles the infrastructure adapters needed by use cases
///
/// This struct represents the application layer's view of infrastructure.
/// Use cases are constructed with domain data, then executed with an Application.
pub struct Application<'a> {
    pub storage: &'a dyn StoragePort,
    pub display: &'a dyn DisplayPort,
    pub log: &'a dyn LogPort,
}

impl<'a> Application<'a> {
    pub fn new(
        storage: &'a dyn StoragePort,
        display: &'a dyn DisplayPort,
        log: &'a dyn LogPort,
    ) -> Self {
        Self {
            storage,
            display,
            log,
        }
    }
}
