// Use case: Start a yak (sugar for SetState with state="wip")

use crate::domain::views::Message;
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
