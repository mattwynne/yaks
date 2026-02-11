// Use case: Edit a yak's context

use anyhow::Result;

use super::{Application, UseCase};

pub struct EditContext {
    name: String,
}

impl EditContext {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Get current context
        let current_context = app.store.get_yak(&self.name)?.context.unwrap_or_default();

        // Request new content via input
        if let Some(content) = app.input.request_content(Some(&current_context), None)? {
            app.with_yak(&self.name, |yak| yak.update_context(content))?;
        }

        Ok(())
    }
}

impl UseCase for EditContext {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
