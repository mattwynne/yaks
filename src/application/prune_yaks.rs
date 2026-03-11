// Use case: Remove all done yaks

use crate::adapters::views::Message;
use anyhow::Result;

use super::{Application, UseCase};

pub struct PruneYaks {
    exclude_tag: Option<String>,
}

impl PruneYaks {
    pub fn new() -> Self {
        Self { exclude_tag: None }
    }

    pub fn with_exclude_tag(mut self, tag: &str) -> Self {
        self.exclude_tag = Some(tag.to_string());
        self
    }
}

impl Default for PruneYaks {
    fn default() -> Self {
        Self::new()
    }
}

impl UseCase for PruneYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let before_count = app.store.list_yaks()?.len();

        app.with_yak_map(|yak_map| yak_map.prune(self.exclude_tag.as_deref()))?;

        let after_count = app.store.list_yaks()?.len();
        let pruned = before_count - after_count;

        if pruned == 0 {
            app.display
                .message(&Message::Success("No done yaks to prune".into()));
        } else {
            app.display
                .message(&Message::Success(format!("Pruned {} done yak(s)", pruned)));
        }

        Ok(())
    }
}
