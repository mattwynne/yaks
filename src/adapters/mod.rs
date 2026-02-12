// Adapters - implementations of port traits for specific technologies

pub mod cli;
pub mod event_store;
pub mod input;
pub mod storage;
pub mod sync;

// Re-export commonly used adapters for external use
#[allow(unused_imports)]
pub use cli::{ConsoleDisplay, InMemoryDisplay};
#[allow(unused_imports)]
pub use event_store::{GitEventStore, InMemoryEventStore};
#[allow(unused_imports)]
pub use input::{ConsoleInput, InMemoryInput};
#[allow(unused_imports)]
pub use storage::{DirectoryStorage, InMemoryStorage};
