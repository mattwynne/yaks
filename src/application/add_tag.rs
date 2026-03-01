// Use case: Add tags to a yak

use anyhow::Result;

use super::{Application, UseCase};

pub struct AddTag {
    name: String,
    tags: Vec<String>,
}

impl AddTag {
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
        let mut tag_set: Vec<String> = existing
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

        // Add new tags, deduplicating
        for tag in &self.tags {
            if !tag_set.contains(tag) {
                tag_set.push(tag.clone());
            }
        }

        let content = tag_set.join("\n");
        app.with_yak_map(|yak_map| yak_map.update_field(id.clone(), "tags".to_string(), content))?;

        app.display.success(&format!("Tagged '{}'", self.name));

        Ok(())
    }
}

impl UseCase for AddTag {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
