// Use case: Remove a yak

use anyhow::Result;

use super::{Application, UseCase};

pub struct RemoveYak {
    name: String,
}

impl RemoveYak {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &Application) -> Result<()> {
        // Resolve yak name (exact or fuzzy match)
        let resolved_name = app.storage.find_yak(&self.name)?;

        // Delete the yak
        app.storage.delete_yak(&resolved_name)?;

        // Log the command
        app.log.log_command(&format!("rm {}", self.name))?;

        Ok(())
    }
}

impl UseCase for RemoveYak {
    fn execute(&self, app: &Application) -> Result<()> {
        Self::execute(self, app)
    }
}
