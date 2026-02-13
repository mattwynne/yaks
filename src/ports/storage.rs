// Write-side storage port - abstraction for yak mutation

use anyhow::Result;

pub trait WriteYakStore {
    /// Create a new yak
    fn create_yak(&self, name: &str) -> Result<()>;

    /// Delete a yak
    fn delete_yak(&self, name: &str) -> Result<()>;

    /// Rename a yak
    fn rename_yak(&self, from: &str, to: &str) -> Result<()>;

    /// Write a field for a yak
    /// Returns error if field cannot be written
    fn write_field(&self, yak_name: &str, field_name: &str, content: &str) -> Result<()>;
}
