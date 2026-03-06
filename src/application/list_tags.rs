// Use case: List tags on a yak

use crate::domain::field::TAGS_FIELD;
use crate::domain::format_tag;
use crate::domain::views::Message;
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
}

impl UseCase for ListTags {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        // Read existing tags
        let existing = app.store.read_field(&id, TAGS_FIELD).unwrap_or_default();
        let tags: Vec<&str> = existing.lines().filter(|l| !l.is_empty()).collect();

        for tag in tags {
            app.display.message(&Message::Info(format_tag(tag)));
        }

        Ok(())
    }
}
