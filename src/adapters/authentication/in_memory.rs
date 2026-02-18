// In-memory authentication adapter for testing

use crate::domain::event_metadata::Author;
use crate::domain::ports::AuthenticationPort;

/// In-memory authentication adapter for testing
///
/// Returns a fixed test author without reading git config.
pub struct InMemoryAuthentication;

impl InMemoryAuthentication {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InMemoryAuthentication {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthenticationPort for InMemoryAuthentication {
    fn current_author(&self) -> Author {
        Author {
            name: "test".to_string(),
            email: "test@test.com".to_string(),
        }
    }
}
