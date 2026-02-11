// Use case: Move/rename a yak

use crate::domain::validate_yak_name;
use anyhow::Result;

use super::{Application, UseCase};

pub struct MoveYak {
    from: String,
    to: String,
}

impl MoveYak {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    pub fn execute(&self, app: &Application) -> Result<()> {
        // Validate target yak name
        validate_yak_name(&self.to).map_err(|e| anyhow::anyhow!(e))?;

        // Resolve source yak name (exact or fuzzy match)
        let resolved_from = app.storage.find_yak(&self.from)?;

        // Check if destination is an existing yak (parent-only move)
        let actual_destination = if app.storage.get_yak(&self.to).is_ok() {
            // Destination exists - treat as parent-only move
            // Extract the base name from the source (everything after last '/')
            let base_name = resolved_from.rsplit('/').next().unwrap();
            format!("{}/{}", self.to, base_name)
        } else {
            self.to.clone()
        };

        // Rename the yak
        app.storage
            .rename_yak(&resolved_from, &actual_destination)?;

        // Log the command
        app.log
            .log_command(&format!("mv {} {}", self.from, self.to))?;

        Ok(())
    }
}

impl UseCase for MoveYak {
    fn execute(&self, app: &Application) -> Result<()> {
        Self::execute(self, app)
    }
}
