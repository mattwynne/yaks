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
        let id = app.store.fuzzy_find_yak_id(&self.name)?;
        // Get current context
        let current_context = app.store.get_yak(&id)?.context.unwrap_or_default();

        // Request new content via input
        if let Some(content) = app.input.request_content(Some(&current_context), None)? {
            app.with_yak_map(|yak_map| yak_map.update_context(id.clone(), content))?;
        }

        Ok(())
    }
}

impl UseCase for EditContext {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
