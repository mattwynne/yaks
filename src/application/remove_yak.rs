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
        let resolved = app.store.find_yak(&self.name)?;
        app.with_yak_map(|yak_map| yak_map.remove_yak(resolved))
    }
}

impl UseCase for RemoveYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
