// Use case: Remove a yak

use anyhow::Result;

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
        app.with_yak_map(|yak_map| {
            yak_map.remove_yak(self.name.clone())
        })
    }
}

impl UseCase for RemoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
