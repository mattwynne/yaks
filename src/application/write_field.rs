// Use case: Write to a yak field

use crate::domain::validate_field_name;
use anyhow::{Context as AnyhowContext, Result};
use std::io::{self, Read};

use super::Application;

pub struct WriteField {
    name: String,
    field: String,
}

impl WriteField {
    pub fn new(name: &str, field: &str) -> Self {
        Self {
            name: name.to_string(),
            field: field.to_string(),
        }
    }

    pub fn execute(&self, app: &Application) -> Result<()> {
        // Validate field name
        validate_field_name(&self.field)?;

        // Resolve yak name (exact or fuzzy match)
        let resolved_name = app.storage.find_yak(&self.name)?;

        // Read content from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;

        // Write to field
        app.storage
            .write_field(&resolved_name, &self.field, &buffer)?;

        // Log the command
        app.log
            .log_command(&format!("field {} {}", self.name, self.field))?;

        Ok(())
    }
}
