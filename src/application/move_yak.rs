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

        let id = app.store.fuzzy_find_yak_id(&self.from)?;

        // Check if destination is an existing yak (parent-only move)
        let actual_destination = if app.store.yak_exists(&self.to) {
            let source_yak = app.store.get_yak(&id)?;
            let base_name = source_yak.name.as_str().rsplit('/').next().unwrap();
            format!("{}/{}", self.to, base_name)
        } else {
            self.to.clone()
        };

        app.with_yak_map(|yak_map| yak_map.move_yak(id, actual_destination))
    }
}

impl UseCase for MoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
