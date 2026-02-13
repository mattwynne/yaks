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
        let resolved = app.store.find_yak(&self.name)?;
        let yak = app.store.get_yak(&resolved)?;

        // Display the header (yak name)
        app.display.info(&yak.name);

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
