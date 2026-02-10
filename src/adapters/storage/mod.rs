// Storage adapters - implementations for different storage backends

pub mod directory;
pub mod memory;

pub use directory::DirectoryStorage;
pub use memory::InMemoryStorage;
