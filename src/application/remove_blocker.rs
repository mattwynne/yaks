use crate::adapters::views::Message;
use crate::domain::yak_map::RemoveBlockerOutcome;
use anyhow::Result;

use super::{Application, UseCase};

pub struct RemoveBlocker {
    target: String,
    blocker: String,
}

impl RemoveBlocker {
    pub fn new(target: &str, blocker: &str) -> Self {
        Self {
            target: target.to_string(),
            blocker: blocker.to_string(),
        }
    }
}

impl UseCase for RemoveBlocker {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let target_id = app.resolve_yak_id(&self.target)?;
        let blocker_id = app.resolve_yak_id(&self.blocker)?;
        let outcome =
            app.with_yak_map_result(|yak_map| yak_map.remove_blocker(target_id, blocker_id))?;
        match outcome {
            RemoveBlockerOutcome::Removed => app.display.message(&Message::Success(format!(
                "Removed blocker '{}' from '{}'",
                self.blocker, self.target
            ))),
            RemoveBlockerOutcome::NotPresent => app.display.message(&Message::Hint(format!(
                "No active explicit blocker '{}' on '{}'; nothing changed",
                self.blocker, self.target
            ))),
        }
        Ok(())
    }
}
