// Use case: Remove a yak

use anyhow::Result;
use crate::domain::YakEvent;

use super::{Application, UseCase};

pub struct RemoveYak {
    name: String,
}

impl RemoveYak {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Verify yak exists first
        let mut yak = app.store.get_yak(&self.name)?;

        // Emit Removed event
        yak.pending_events.push(YakEvent::Removed {
            name: yak.name.clone(),
        });

        // Publish the event
        for event in yak.take_events() {
            app.event_bus.publish(event)?;
        }

        Ok(())
    }
}

impl UseCase for RemoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
