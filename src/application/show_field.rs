// Use case: Show a yak field

use crate::domain::validate_field_name_format;
use anyhow::Result;

use super::{Application, UseCase};

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

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Validate field name format (allow reserved fields for reading)
        validate_field_name_format(&self.field)?;

        // Find yak (handles fuzzy matching)
        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        // Get yak to display name
        let yak = app.store.get_yak(&id)?;

        // Read field content
        let content = app.store.read_field(&id, &self.field)?;

        // Display yak name and field content
        app.display.success(yak.name.as_str());
        app.display.info("");
        app.display.info(&content);

        Ok(())
    }
}

impl UseCase for ShowField {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
