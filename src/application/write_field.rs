// Use case: Write to a yak field

use crate::domain::validate_field_name;
use anyhow::Result;

use super::{Application, UseCase};

pub struct WriteField {
    name: String,
    field: String,
    content: Option<String>,
}

impl WriteField {
    pub fn new(name: &str, field: &str) -> Self {
        Self {
            name: name.to_string(),
            field: field.to_string(),
            content: None,
        }
    }

    /// Pre-set content (bypasses stdin/editor input)
    pub fn with_content(mut self, content: &str) -> Self {
        self.content = Some(content.to_string());
        self
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Validate field name
        validate_field_name(&self.field)?;

        // Use pre-set content, or request via input port
        let content = if let Some(ref content) = self.content {
            Some(content.clone())
        } else {
            app.input.request_content(None, None)?
        };

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
