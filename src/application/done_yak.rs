// Use case: Mark a yak as done (sugar for SetState with state="done")

use crate::domain::views::Message;
use anyhow::Result;

use super::{Application, SetState, UseCase};

pub struct DoneYak {
    name: String,
    recursive: bool,
}

impl DoneYak {
    pub fn new(name: &str, recursive: bool) -> Self {
        Self {
            name: name.to_string(),
            recursive,
        }
    }
}

impl UseCase for DoneYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        SetState::new(&self.name, "done")
            .with_recursive(self.recursive)
            .with_silent(true)
            .execute(app)?;

        if self.recursive {
            app.display.message(&Message::Success(format!(
                "Marked '{}' and descendants as done",
                self.name
            )));
        } else {
            app.display
                .message(&Message::Success(format!("Marked '{}' as done", self.name)));
        }

        Ok(())
    }
}
