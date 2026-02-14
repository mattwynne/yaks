// Public library interface for integration tests

pub mod adapters;
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod projections;

pub use infrastructure::EventBus;
