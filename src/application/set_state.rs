// Use case: Set a yak's state

use anyhow::Result;

use super::{Application, UseCase};

pub struct SetState {
    name: String,
    state: String,
}

impl SetState {
    pub fn new(name: &str, state: &str) -> Self {
        Self {
            name: name.to_string(),
            state: state.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        app.with_yak(&self.name, |yak| yak.update_state(self.state.clone()))
    }
}

impl UseCase for SetState {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
