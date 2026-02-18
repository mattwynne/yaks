// Use case: Move a yak in the hierarchy

use anyhow::Result;

use super::{Application, UseCase};

pub struct MoveYak {
    name: String,
    under: Option<String>,
    to_root: bool,
}

impl MoveYak {
    /// Move a yak under a parent (--under flag)
    pub fn under(name: &str, parent: &str) -> Self {
        Self {
            name: name.to_string(),
            under: Some(parent.to_string()),
            to_root: false,
        }
    }

    /// Move a yak to root level (--to-root flag)
    pub fn to_root(name: &str) -> Self {
        Self {
            name: name.to_string(),
            under: None,
            to_root: true,
        }
    }

    /// Both flags specified (should error)
    pub fn under_and_to_root(name: &str, parent: &str) -> Self {
        Self {
            name: name.to_string(),
            under: Some(parent.to_string()),
            to_root: true,
        }
    }

    /// No flags specified (should error)
    pub fn no_flags(name: &str) -> Self {
        Self {
            name: name.to_string(),
            under: None,
            to_root: false,
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Validate: exactly one of --under or --to-root must be provided
        if self.under.is_some() && self.to_root {
            anyhow::bail!("Cannot use both --under and --to-root. Use one or the other.");
        }
        if self.under.is_none() && !self.to_root {
            anyhow::bail!("Must specify either --under <parent> or --to-root.");
        }

        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        if self.to_root {
            // Move to root: set parent to None
            app.with_yak_map(|yak_map| yak_map.move_yak_to(id, None))
        } else {
            // Move under parent: resolve parent by fuzzy match
            let parent_name = self.under.as_ref().unwrap();
            let parent_id = app.store.fuzzy_find_yak_id(parent_name)?;
            app.with_yak_map(|yak_map| yak_map.move_yak_to(id, Some(parent_id)))
        }
    }
}

impl UseCase for MoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
