// Use case: Remove all done yaks

use anyhow::Result;

use super::{Application, UseCase};

pub struct PruneYaks {}

impl PruneYaks {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PruneYaks {
    fn default() -> Self {
        Self::new()
    }
}

impl PruneYaks {
    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let before_count = app.store.list_yaks()?.len();

        app.with_yak_map(|yak_map| yak_map.prune())?;

        let after_count = app.store.list_yaks()?.len();
        let pruned = before_count - after_count;

        if pruned == 0 {
            app.display.success("No done yaks to prune");
        } else {
            app.display
                .success(&format!("Pruned {} done yak(s)", pruned));
        }

        Ok(())
    }
}

impl UseCase for PruneYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
