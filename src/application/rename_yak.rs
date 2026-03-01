// Use case: Rename a yak (change name without moving)

use crate::domain::validate_yak_name;
use anyhow::Result;

use super::{Application, UseCase};

pub struct RenameYak {
    from: String,
    to: String,
}

impl RenameYak {
    pub fn new(from: &str, to: &str) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        validate_yak_name(&self.to).map_err(|e| anyhow::anyhow!(e))?;

        let id = app.store.fuzzy_find_yak_id(&self.from)?;

        app.with_yak_map(|yak_map| yak_map.rename_yak(id, self.to.clone()))?;

        app.display
            .success(&format!("Renamed '{}' to '{}'", self.from, self.to));

        Ok(())
    }
}

impl UseCase for RenameYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
