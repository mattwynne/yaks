use crate::adapters::views::Message;
use crate::domain::yak_map::AddBlockerOutcome;
use anyhow::{bail, Result};

use super::{Application, UseCase};

pub struct AddBlocker {
    target: String,
    blocker: Option<String>,
    reason: Option<String>,
}

impl AddBlocker {
    pub fn new(target: &str, blocker: &str) -> Self {
        Self {
            target: target.to_string(),
            blocker: Some(blocker.to_string()),
            reason: None,
        }
    }

    pub fn manual(target: &str, reason: &str) -> Self {
        Self {
            target: target.to_string(),
            blocker: None,
            reason: Some(reason.to_string()),
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
        let reason = self.reason.clone();
        let outcome = if let Some(blocker) = &self.blocker {
            let blocker_id = app.resolve_yak_id(blocker)?;
            app.with_yak_map_result(|yak_map| yak_map.add_blocker(target_id, blocker_id, reason))?
        } else {
            let reason = reason.unwrap_or_default();
            app.with_yak_map_result(|yak_map| yak_map.add_manual_blocker(target_id, reason))?
        };
        match outcome {
            AddBlockerOutcome::Added => {
                app.display.message(&Message::Success(match &self.blocker {
                    Some(blocker) => format!("Marked '{}' blocked by '{}'", self.target, blocker),
                    None => format!("Added manual blocker for '{}'", self.target),
                }))
            }
            AddBlockerOutcome::Updated => {
                app.display.message(&Message::Success(match &self.blocker {
                    Some(blocker) => format!("Updated blocker '{}' for '{}'", blocker, self.target),
                    None => format!("Updated manual blocker for '{}'", self.target),
                }))
            }
            AddBlockerOutcome::AlreadyExplicit => bail!(match &self.blocker {
                Some(blocker) => format!(
                    "'{}' already blocks '{}'; nothing changed",
                    blocker, self.target
                ),
                None => format!(
                    "manual blocker already present on '{}'; nothing changed",
                    self.target
                ),
            }),
            AddBlockerOutcome::AlreadyImpliedByHierarchy => {
                app.display.message(&Message::Hint(format!(
                    "'{}' already blocks '{}' through hierarchy; no explicit blocker added",
                    self.blocker.clone().unwrap_or_default(),
                    self.target
                )))
            }
        }
        Ok(())
    }
}
