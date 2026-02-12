// Storage adapters - implementations for different storage backends

pub mod directory;
#[cfg(any(test, feature = "test-support"))]
pub mod memory;

pub use directory::DirectoryStorage;
#[cfg(any(test, feature = "test-support"))]
pub use memory::InMemoryStorage;
