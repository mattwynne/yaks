// Use case: Show a yak field

use crate::domain::validate_field_name;
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
        // Validate field name
        validate_field_name(&self.field)?;

        // Note: This use case needs access to StoragePort methods (find_yak, read_field)
        // that aren't on Store trait. This is a temporary workaround.
        // TODO: Consider adding field reading to Store trait or creating a separate port

        // For now, show a simplified version using Store
        let _yak = app.store.get_yak(&self.name)?;

        // Field reading not yet supported in event-sourced model
        // This will need to be addressed in future refactoring
        anyhow::bail!("Field reading not yet implemented in event-sourced model")
    }
}

impl UseCase for ShowField {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
