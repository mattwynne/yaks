// Null input adapter - always returns None without user interaction
//
// Used for non-interactive operations like replay/reset where
// no user input should be solicited.

use crate::domain::ports::InputPort;
use anyhow::Result;

/// Null input adapter that never prompts the user
///
/// Returns `Ok(None)` for all input requests. Use this when
/// running operations that must complete without user interaction
/// (e.g., hard reset replay).
pub struct NullInput;

impl InputPort for NullInput {
    fn request_content(
        &self,
        _initial_content: Option<&str>,
        _template: Option<&str>,
    ) -> Result<Option<String>> {
        Ok(None)
    }

    fn confirm(&self, _message: &str) -> Result<bool> {
        // Non-interactive: always decline
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none() {
        let input = NullInput;
        assert_eq!(input.request_content(None, None).unwrap(), None);
    }

    #[test]
    fn ignores_initial_content_and_template() {
        let input = NullInput;
        assert_eq!(
            input
                .request_content(Some("content"), Some("template"))
                .unwrap(),
            None
        );
    }

    #[test]
    fn confirm_returns_false() {
        let input = NullInput;
        assert!(!input.confirm("proceed?").unwrap());
    }
}
