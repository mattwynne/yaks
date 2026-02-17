// Use case: Show yak context

use anyhow::Result;

use super::{Application, UseCase};

pub struct ShowContext {
    name: String,
}

impl ShowContext {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;
        let yak = app.store.get_yak(&id)?;

        // Display the header (yak name)
        app.display.info(yak.name.as_str());

        // Display a blank line if there's content
        if let Some(context) = &yak.context {
            if !context.is_empty() {
                app.display.info("");
                // Display the context
                app.display.info(context);
            }
        }

        Ok(())
    }
}

impl UseCase for ShowContext {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
