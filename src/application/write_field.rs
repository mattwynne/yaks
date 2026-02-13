// Use case: Write to a yak field

use crate::domain::validate_field_name;
use anyhow::{Context as AnyhowContext, Result};

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

        // Request content via input port
        let content = app
            .input
            .request_content(None, None)?
            .context("No content provided on stdin")?;

        app.with_yak(&self.name, |yak| {
            yak.update_field(self.field.clone(), content)
        })
    }
}

impl UseCase for WriteField {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
