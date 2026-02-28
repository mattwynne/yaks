// Use case: List tags on a yak

use crate::domain::format_tag;
use anyhow::Result;

use super::{Application, UseCase};

pub struct ListTags {
    name: String,
}

impl ListTags {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        // Read existing tags
        let existing = app.store.read_field(&id, "tags").unwrap_or_default();
        let tags: Vec<&str> = existing.lines().filter(|l| !l.is_empty()).collect();

        for tag in tags {
            app.display.info(&format_tag(tag));
        }

        Ok(())
    }
}

impl UseCase for ListTags {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
