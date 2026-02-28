// Use case: Edit a yak's context

use anyhow::Result;

use super::{Application, UseCase};

pub struct EditContext {
    name: String,
    initial_content: Option<String>,
}

impl EditContext {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            initial_content: None,
        }
    }

    /// Pre-set initial content for the editor (e.g., from stdin).
    /// When set, this content is used as the editor's starting text
    /// instead of the existing context from the store.
    pub fn with_initial_content(mut self, content: &str) -> Self {
        self.initial_content = Some(content.to_string());
        self
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        // Use pre-provided initial content, or fall back to existing context
        let current_context = match &self.initial_content {
            Some(content) => content.clone(),
            None => app.store.get_yak(&id)?.context.unwrap_or_default(),
        };

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
