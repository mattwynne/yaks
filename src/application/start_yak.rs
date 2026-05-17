// Use case: Start a yak (sugar for SetState with state="wip")

use crate::adapters::views::Message;
use anyhow::Result;

use super::{Application, SetState, UseCase};

pub struct StartYak {
    name: String,
    recursive: bool,
}

impl StartYak {
    pub fn new(name: &str, recursive: bool) -> Self {
        Self {
            name: name.to_string(),
            recursive,
        }
    }
}

impl UseCase for StartYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.resolve_yak_id(&self.name)?;
        if !self.recursive {
            app.with_yak_map_result(|yak_map| yak_map.ensure_ready_to_start(&id))?;
        }

        SetState::new(&self.name, "wip")
            .with_recursive(self.recursive)
            .with_silent(true)
            .execute(app)?;

        if self.recursive {
            app.display.message(&Message::Success(format!(
                "Started '{}' and descendants",
                self.name
            )));
        } else {
            app.display
                .message(&Message::Success(format!("Started '{}'", self.name)));
        }

        Ok(())
    }
}
