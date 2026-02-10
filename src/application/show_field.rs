// Use case: Show a yak field

use crate::domain::validate_field_name;
use anyhow::Result;

use super::Application;

pub struct ShowField {
    name: String,
    field: String,
}

impl ShowField {
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

        // Read field content
        let content = app.storage.read_field(&resolved_name, &self.field)?;

        // Output the yak name and content (similar to context --show)
        app.display
            .info(&format!("{}\n\n{}", resolved_name, content));

        // Log the command
        app.log
            .log_command(&format!("field {} {} --show", self.name, self.field))?;

        Ok(())
    }
}
