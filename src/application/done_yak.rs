// Use case: Mark a yak as done or undone

use crate::domain::STATE_FIELD;
use anyhow::Result;

use super::{Application, UseCase};

pub struct DoneYak {
    name: String,
    undo: bool,
    recursive: bool,
}

impl DoneYak {
    pub fn new(name: &str, undo: bool, recursive: bool) -> Self {
        Self {
            name: name.to_string(),
            undo,
            recursive,
        }
    }

    pub fn execute(&self, app: &Application) -> Result<()> {
        // Resolve yak name (exact or fuzzy match)
        let resolved_name = app.storage.find_yak(&self.name)?;

        // If marking as done (not undo) and not recursive, check for incomplete children
        if !self.undo && !self.recursive {
            let all_yaks = app.storage.list_yaks()?;
            let has_incomplete_children = all_yaks
                .iter()
                .any(|yak| yak.name.starts_with(&format!("{resolved_name}/")) && !yak.done);

            if has_incomplete_children {
                anyhow::bail!("cannot mark '{resolved_name}' as done - it has incomplete children");
            }
        }

        // If recursive, mark all children as done too
        if self.recursive && !self.undo {
            let all_yaks = app.storage.list_yaks()?;
            let children: Vec<String> = all_yaks
                .iter()
                .filter(|yak| {
                    yak.name == resolved_name || yak.name.starts_with(&format!("{resolved_name}/"))
                })
                .map(|yak| yak.name.clone())
                .collect();

            for child_name in children {
                app.storage.write_field(&child_name, STATE_FIELD, "done")?;
            }
        } else {
            // Mark just this yak as done/undone
            let new_state = if self.undo { "todo" } else { "done" };
            app.storage
                .write_field(&resolved_name, STATE_FIELD, new_state)?;
        }

        // Log the command
        let command = if self.undo {
            format!("done --undo {}", self.name)
        } else if self.recursive {
            format!("done --recursive {}", self.name)
        } else {
            format!("done {}", self.name)
        };
        app.log.log_command(&command)?;

        Ok(())
    }
}

impl UseCase for DoneYak {
    fn execute(&self, app: &Application) -> Result<()> {
        Self::execute(self, app)
    }
}
