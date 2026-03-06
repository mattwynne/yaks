// Use case: Remove tags from a yak

use crate::domain::field::TAGS_FIELD;
use crate::domain::views::Message;
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
}

impl UseCase for RemoveTag {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        // Read existing tags
        let existing = app.store.read_field(&id, TAGS_FIELD).unwrap_or_default();
        let tag_set: Vec<String> = existing
            .lines()
            .filter(|l| !l.is_empty())
            .filter(|l| !self.tags.contains(&l.to_string()))
            .map(|l| l.to_string())
            .collect();

        let content = tag_set.join("\n");
        app.with_yak_map(|yak_map| {
            yak_map.update_field(id.clone(), TAGS_FIELD.to_string(), content)
        })?;

        app.display.message(&Message::Success(format!(
            "Removed tag from '{}'",
            self.name
        )));

        Ok(())
    }
}
