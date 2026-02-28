// In-memory input adapter for testing

use crate::domain::ports::InputPort;
use anyhow::Result;
use std::cell::RefCell;

/// In-memory input adapter for testing
///
/// Returns predefined content without any console interaction.
/// Useful for unit tests that need to simulate user input.
pub struct InMemoryInput {
    content: RefCell<Option<String>>,
    confirm_response: RefCell<bool>,
}

impl InMemoryInput {
    /// Create a new InMemoryInput that returns None (no input)
    pub fn new() -> Self {
        Self {
            content: RefCell::new(None),
            confirm_response: RefCell::new(true),
        }
    }

    /// Create a new InMemoryInput with predefined content
    pub fn with_content(content: String) -> Self {
        Self {
            content: RefCell::new(Some(content)),
            confirm_response: RefCell::new(true),
        }
    }

    /// Set the content that will be returned by request_content
    pub fn set_content(&self, content: Option<String>) {
        *self.content.borrow_mut() = content;
    }

    /// Set the response for confirm() calls
    pub fn set_confirm(&self, response: bool) {
        *self.confirm_response.borrow_mut() = response;
    }
}

impl Default for InMemoryInput {
    fn default() -> Self {
        Self::new()
    }
}

impl InputPort for InMemoryInput {
    fn request_content(
        &self,
        _initial_content: Option<&str>,
        _template: Option<&str>,
    ) -> Result<Option<String>> {
        // Return the predefined content
        Ok(self.content.borrow().clone())
    }

    fn confirm(&self, _message: &str) -> Result<bool> {
        Ok(*self.confirm_response.borrow())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_returns_none() {
        let input = InMemoryInput::new();
        let result = input.request_content(None, None).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_with_content_returns_content() {
        let input = InMemoryInput::with_content("test content".to_string());
        let result = input.request_content(None, None).unwrap();
        assert_eq!(result, Some("test content".to_string()));
    }

    #[test]
    fn test_set_content_updates_content() {
        let input = InMemoryInput::new();
        input.set_content(Some("updated".to_string()));
        let result = input.request_content(None, None).unwrap();
        assert_eq!(result, Some("updated".to_string()));
    }

    #[test]
    fn test_ignores_initial_content_and_template() {
        let input = InMemoryInput::with_content("predefined".to_string());
        let result = input
            .request_content(Some("initial"), Some("template"))
            .unwrap();
        assert_eq!(result, Some("predefined".to_string()));
    }

    #[test]
    fn test_confirm_defaults_to_true() {
        let input = InMemoryInput::new();
        assert!(input.confirm("proceed?").unwrap());
    }

    #[test]
    fn test_confirm_can_be_set_to_false() {
        let input = InMemoryInput::new();
        input.set_confirm(false);
        assert!(!input.confirm("proceed?").unwrap());
    }
}
