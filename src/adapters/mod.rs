// Adapters - implementations of port traits for specific technologies

pub mod cli;
pub mod log;
pub mod storage;
pub mod sync;

// Re-export commonly used adapters for external use
#[allow(unused_imports)]
pub use cli::{ConsoleOutput, InMemoryOutput};
#[allow(unused_imports)]
pub use log::{GitLog, InMemoryLog};
#[allow(unused_imports)]
pub use storage::{DirectoryStorage, InMemoryStorage};
