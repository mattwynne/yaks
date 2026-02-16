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
        // Validate each segment of the target name
        for segment in self.to.split('/') {
            validate_yak_name(segment).map_err(|e| anyhow::anyhow!(e))?;
        }

        let resolved_from = app.store.find_yak(&self.from)?;

        // Check if destination is an existing yak (parent-only move)
        let actual_destination = if app.store.yak_exists(&self.to) {
            // Destination exists - treat as parent-only move
            // Extract the base name from the source (everything after last '/')
            let base_name = resolved_from.rsplit('/').next().unwrap();
            format!("{}/{}", self.to, base_name)
        } else {
            self.to.clone()
        };

        app.with_yak_map(|yak_map| yak_map.move_yak(resolved_from, actual_destination))
    }
}

impl UseCase for MoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
