// Use case: Remove all done yaks

use crate::domain::YakEvent;
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
        let yaks = app.store.list_yaks()?;

        for yak in yaks.iter().filter(|y| y.is_done()) {
            let mut yak_to_remove = yak.clone();
            yak_to_remove.pending_events.push(YakEvent::Removed {
                name: yak_to_remove.name.clone(),
            });

            for event in yak_to_remove.take_events() {
                app.event_bus.publish(event)?;
            }
        }

        Ok(())
    }
}

impl UseCase for PruneYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
