// Use case: Write to a yak field

use crate::domain::validate_field_name;
use anyhow::Result;

use super::{Application, UseCase};

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

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Validate field name
        validate_field_name(&self.field)?;

        // Request content via input port (before closure)
        let content = app.input.request_content(None, None)?;

        // No content means no-op (e.g., empty piped stdin)
        if let Some(content) = content {
            let id = app.store.fuzzy_find_yak_id(&self.name)?;
            let field = self.field.clone();
            app.with_yak_map(|yak_map| yak_map.update_field(id, field, content))
        } else {
            Ok(())
        }
    }
}

impl UseCase for WriteField {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
