// Use case: Start a yak (sugar for SetState with state="wip")

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

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        SetState::new(&self.name, "wip")
            .with_recursive(self.recursive)
            .execute(app)
    }
}

impl UseCase for StartYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
