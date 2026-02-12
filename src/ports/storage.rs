// Storage port trait - abstraction for yak persistence

use crate::domain::Yak;
use anyhow::Result;

pub trait StoragePort {
    /// Create a new yak
    fn create_yak(&self, name: &str) -> Result<()>;

    /// Get a yak by name
    fn get_yak(&self, name: &str) -> Result<Yak>;

    /// List all yaks
    fn list_yaks(&self) -> Result<Vec<Yak>>;

    /// Delete a yak
    fn delete_yak(&self, name: &str) -> Result<()>;

    /// Rename a yak
    fn rename_yak(&self, from: &str, to: &str) -> Result<()>;

    /// Find a yak by name or fuzzy match
    /// Returns the exact name if found, or a unique fuzzy match
    /// Returns error if not found or ambiguous
    fn find_yak(&self, name: &str) -> Result<String>;

    /// Write a field for a yak
    /// Returns error if field cannot be written
    fn write_field(&self, yak_name: &str, field_name: &str, content: &str) -> Result<()>;

    /// Read a field for a yak
    /// Returns error if field doesn't exist
    fn read_field(&self, yak_name: &str, field_name: &str) -> Result<String>;
}
