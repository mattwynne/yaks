// Use case: Mark a yak as done (sugar for SetState with state="done")

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

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        SetState::new(&self.name, "done")
            .with_recursive(self.recursive)
            .execute(app)
    }
}

impl UseCase for DoneYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
