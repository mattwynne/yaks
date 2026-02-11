// Application struct - bundles infrastructure adapters for use case execution

use crate::ports::{DisplayPort, InputPort, LogPort, StoragePort};
use anyhow::Result;

use super::UseCase;

/// Application bundles the infrastructure adapters needed by use cases
///
/// This struct represents the application layer's view of infrastructure.
/// Use cases are constructed with domain data, then executed with an Application.
pub struct Application<'a> {
    pub storage: &'a dyn StoragePort,
    pub display: &'a dyn DisplayPort,
    pub log: &'a dyn LogPort,
    pub input: &'a dyn InputPort,
}

impl<'a> Application<'a> {
    pub fn new(
        storage: &'a dyn StoragePort,
        display: &'a dyn DisplayPort,
        log: &'a dyn LogPort,
        input: &'a dyn InputPort,
    ) -> Self {
        Self {
            storage,
            display,
            log,
            input,
        }
    }

    /// Execute a use case with this application's infrastructure
    ///
    /// # Example
    /// ```ignore
    /// let app = Application::new(&storage, &display, &log, &input);
    /// app.handle(AddYak::new("my yak"))?;
    /// ```
    pub fn handle<U: UseCase>(&self, use_case: U) -> Result<()> {
        use_case.execute(self)
    }
}
