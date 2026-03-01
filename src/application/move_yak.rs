// Use case: Move a yak in the hierarchy

use anyhow::Result;

use super::{Application, UseCase};

pub enum MoveTarget {
    Under(String),
    ToRoot,
}

pub struct MoveYak {
    name: String,
    target: MoveTarget,
}

impl MoveYak {
    /// Move a yak under a parent (--under flag)
    pub fn under(name: &str, parent: &str) -> Self {
        Self {
            name: name.to_string(),
            target: MoveTarget::Under(parent.to_string()),
        }
    }

    /// Move a yak to root level (--to-root flag)
    pub fn to_root(name: &str) -> Self {
        Self {
            name: name.to_string(),
            target: MoveTarget::ToRoot,
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        match &self.target {
            MoveTarget::ToRoot => {
                app.with_yak_map(|yak_map| yak_map.move_yak_to(id, None))?;
                app.display
                    .success(&format!("Moved '{}' to root", self.name));
            }
            MoveTarget::Under(parent_name) => {
                let parent_id = app.store.fuzzy_find_yak_id(parent_name)?;
                app.with_yak_map(|yak_map| yak_map.move_yak_to(id, Some(parent_id)))?;
                app.display
                    .success(&format!("Moved '{}' under '{}'", self.name, parent_name));
            }
        }

        Ok(())
    }
}

impl UseCase for MoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
