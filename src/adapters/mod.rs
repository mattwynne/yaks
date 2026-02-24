// Adapters - implementations of port traits for specific technologies

pub mod authentication;
pub mod event_store;
pub mod user_display;
pub mod user_input;
pub mod yak_store;

// Re-export test adapters for use in tests across the crate
#[cfg(any(test, feature = "test-support"))]
pub use authentication::InMemoryAuthentication;
#[cfg(any(test, feature = "test-support"))]
pub use event_store::InMemoryEventStore;
#[cfg(any(test, feature = "test-support"))]
pub use user_display::{make_test_display, TestBuffer};
#[cfg(any(test, feature = "test-support"))]
pub use user_input::InMemoryInput;
#[cfg(any(test, feature = "test-support"))]
pub use yak_store::InMemoryStorage;
