// Port for user input/editing abstraction
//
// This port abstracts away the mechanism of getting content from users.
// Different adapters can implement this for:
// - Console (stdin, editor)
// - Web forms
// - GUI dialogs
// - Test fixtures

use anyhow::Result;

/// Port for getting user input or editing content
pub trait InputPort {
    /// Request content from the user
    ///
    /// # Arguments
    /// * `initial_content` - Optional starting content (e.g., for editing existing content)
    /// * `template` - Optional template or hints to show the user
    ///
    /// # Returns
    /// * `Ok(Some(content))` - User provided content
    /// * `Ok(None)` - User cancelled or no input needed
    /// * `Err(_)` - Error occurred
    fn request_content(
        &self,
        initial_content: Option<&str>,
        template: Option<&str>,
    ) -> Result<Option<String>>;

    /// Ask the user to confirm an action
    ///
    /// # Arguments
    /// * `message` - The message/question to display
    ///
    /// # Returns
    /// * `Ok(true)` - User confirmed
    /// * `Ok(false)` - User declined
    /// * `Err(_)` - Error occurred
    fn confirm(&self, message: &str) -> Result<bool>;
}
