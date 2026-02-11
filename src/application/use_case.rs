// UseCase trait - defines the interface for all use cases

use anyhow::Result;

use super::Application;

/// Trait for use cases that can be executed with an Application
///
/// All use cases follow the same pattern:
/// 1. Construct with domain data (new)
/// 2. Execute with infrastructure (execute)
pub trait UseCase {
    /// Execute the use case with the application's infrastructure
    fn execute(&self, app: &mut Application) -> Result<()>;
}
