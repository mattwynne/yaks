use crate::adapters::views::Message;
use crate::domain::yak_map::RemoveBlockerOutcome;
use anyhow::Result;

use super::{Application, UseCase};

pub struct RemoveBlocker {
    target: String,
    blocker: Option<String>,
}

impl RemoveBlocker {
    pub fn new(target: &str, blocker: &str) -> Self {
        Self {
            target: target.to_string(),
            blocker: Some(blocker.to_string()),
        }
    }

    pub fn manual(target: &str) -> Self {
        Self {
            target: target.to_string(),
            blocker: None,
        }
    }
}

impl UseCase for RemoveBlocker {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let target_id = app.resolve_yak_id(&self.target)?;
        let outcome = if let Some(blocker) = &self.blocker {
            let blocker_id = app.resolve_yak_id(blocker)?;
            app.with_yak_map_result(|yak_map| yak_map.remove_blocker(target_id, blocker_id))?
        } else {
            app.with_yak_map_result(|yak_map| yak_map.remove_manual_blocker(target_id))?
        };
        match outcome {
            RemoveBlockerOutcome::Removed => {
                app.display.message(&Message::Success(match &self.blocker {
                    Some(blocker) => {
                        format!("Removed blocker '{}' from '{}'", blocker, self.target)
                    }
                    None => format!("Removed manual blocker from '{}'", self.target),
                }))
            }
            RemoveBlockerOutcome::NotPresent => {
                app.display.message(&Message::Hint(match &self.blocker {
                    Some(blocker) => format!(
                        "No active explicit blocker '{}' on '{}'; nothing changed",
                        blocker, self.target
                    ),
                    None => format!(
                        "No active manual blocker on '{}'; nothing changed",
                        self.target
                    ),
                }))
            }
        }
        Ok(())
    }
}
