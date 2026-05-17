use crate::adapters::views::Message;
use crate::domain::yak_map::AddBlockerOutcome;
use anyhow::{bail, Result};

use super::{Application, UseCase};

pub struct AddBlocker {
    target: String,
    blocker: String,
    reason: Option<String>,
}

impl AddBlocker {
    pub fn new(target: &str, blocker: &str) -> Self {
        Self {
            target: target.to_string(),
            blocker: blocker.to_string(),
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: Option<&str>) -> Self {
        self.reason = reason.map(str::to_string);
        self
    }
}

impl UseCase for AddBlocker {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let target_id = app.resolve_yak_id(&self.target)?;
        let blocker_id = app.resolve_yak_id(&self.blocker)?;
        let reason = self.reason.clone();
        let outcome =
            app.with_yak_map_result(|yak_map| yak_map.add_blocker(target_id, blocker_id, reason))?;
        match outcome {
            AddBlockerOutcome::Added => app.display.message(&Message::Success(format!(
                "Marked '{}' blocked by '{}'",
                self.target, self.blocker
            ))),
            AddBlockerOutcome::Updated => app.display.message(&Message::Success(format!(
                "Updated blocker '{}' for '{}'",
                self.blocker, self.target
            ))),
            AddBlockerOutcome::AlreadyExplicit => bail!(
                "'{}' already blocks '{}'; nothing changed",
                self.blocker,
                self.target
            ),
            AddBlockerOutcome::AlreadyImpliedByHierarchy => {
                app.display.message(&Message::Hint(format!(
                    "'{}' already blocks '{}' through hierarchy; no explicit blocker added",
                    self.blocker, self.target
                )))
            }
        }
        Ok(())
    }
}
