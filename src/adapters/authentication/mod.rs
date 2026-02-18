pub mod git;
pub use git::GitAuthentication;

#[cfg(any(test, feature = "test-support"))]
pub mod in_memory;
#[cfg(any(test, feature = "test-support"))]
pub use in_memory::InMemoryAuthentication;
