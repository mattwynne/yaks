// Use case: Prune all done yaks

use anyhow::Result;

use super::{Application, UseCase};

pub struct PruneYaks;

impl PruneYaks {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, app: &Application) -> Result<()> {
        let yaks = app.storage.list_yaks()?;

        // Remove all done yaks
        for yak in yaks {
            if yak.is_done() {
                app.storage.delete_yak(&yak.name)?;
                app.log.log_command(&format!("rm {}", yak.name))?;
            }
        }

        Ok(())
    }
}

impl Default for PruneYaks {
    fn default() -> Self {
        Self::new()
    }
}

impl UseCase for PruneYaks {
    fn execute(&self, app: &Application) -> Result<()> {
        Self::execute(self, app)
    }
}
