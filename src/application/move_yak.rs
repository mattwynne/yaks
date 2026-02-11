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

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Validate target yak name
        validate_yak_name(&self.to).map_err(|e| anyhow::anyhow!(e))?;

        // Check if destination is an existing yak (parent-only move)
        let actual_destination = if app.store.yak_exists(&self.to) {
            // Destination exists - treat as parent-only move
            // Extract the base name from the source (everything after last '/')
            let base_name = self.from.rsplit('/').next().unwrap();
            format!("{}/{}", self.to, base_name)
        } else {
            self.to.clone()
        };

        app.with_yak(&self.from, |yak| yak.move_to(actual_destination))
    }
}

impl UseCase for MoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
