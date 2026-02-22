// Input adapters

mod console_input;
#[cfg(any(test, feature = "test-support"))]
mod in_memory_input;
mod null_input;

pub use console_input::ConsoleInput;
#[cfg(any(test, feature = "test-support"))]
pub use in_memory_input::InMemoryInput;
pub use null_input::NullInput;
