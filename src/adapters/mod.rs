// Adapters - implementations of port traits for specific technologies

pub mod cli;
pub mod event_store;
pub mod input;
pub mod storage;
pub mod sync;

// Re-export test adapters for use in tests across the crate
#[cfg(any(test, feature = "test-support"))]
pub use cli::InMemoryDisplay;
#[cfg(any(test, feature = "test-support"))]
pub use event_store::InMemoryEventStore;
#[cfg(any(test, feature = "test-support"))]
pub use input::InMemoryInput;
#[cfg(any(test, feature = "test-support"))]
pub use storage::InMemoryStorage;
