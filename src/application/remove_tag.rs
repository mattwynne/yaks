// Use case: Remove tags from a yak

use anyhow::Result;

use super::{Application, UseCase};

pub struct RemoveTag {
    name: String,
    tags: Vec<String>,
}

impl RemoveTag {
    pub fn new(name: &str, tags: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            tags,
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        // Read existing tags
        let existing = app.store.read_field(&id, "tags").unwrap_or_default();
        let tag_set: Vec<String> = existing
            .lines()
            .filter(|l| !l.is_empty())
            .filter(|l| !self.tags.contains(&l.to_string()))
            .map(|l| l.to_string())
            .collect();

        let content = tag_set.join("\n");
        app.with_yak_map(|yak_map| yak_map.update_field(id.clone(), "tags".to_string(), content))?;

        app.display
            .success(&format!("Removed tag from '{}'", self.name));

        Ok(())
    }
}

impl UseCase for RemoveTag {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
