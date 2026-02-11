// Use case: Mark a yak as done or undone

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

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Resolve yak name (exact or fuzzy match)
        // Note: Store doesn't have find_yak, so we need to use StoragePort cast
        // This is a temporary workaround until we add find_yak to a proper port
        let resolved_name = {
            let all_yaks = app.store.list_yaks()?;
            let name = &self.name;

            // Try exact match first
            if app.store.yak_exists(name) {
                name.clone()
            } else {
                // Fuzzy match on leaf node
                let matches: Vec<String> = all_yaks
                    .iter()
                    .filter(|yak| {
                        let leaf = yak.name.rsplit('/').next().unwrap_or(&yak.name);
                        leaf.contains(name)
                    })
                    .map(|yak| yak.name.clone())
                    .collect();

                match matches.len() {
                    0 => anyhow::bail!("yak '{name}' not found"),
                    1 => matches[0].clone(),
                    _ => anyhow::bail!("yak name '{name}' is ambiguous"),
                }
            }
        };

        // If marking as done (not undo) and not recursive, check for incomplete children
        if !self.undo && !self.recursive {
            let all_yaks = app.store.list_yaks()?;
            let has_incomplete_children = all_yaks
                .iter()
                .any(|yak| yak.name.starts_with(&format!("{resolved_name}/")) && !yak.is_done());

            if has_incomplete_children {
                anyhow::bail!("cannot mark '{resolved_name}' as done - it has incomplete children");
            }
        }

        // If recursive, mark all children as done too
        if self.recursive && !self.undo {
            let all_yaks = app.store.list_yaks()?;
            let children: Vec<String> = all_yaks
                .iter()
                .filter(|yak| {
                    yak.name == resolved_name || yak.name.starts_with(&format!("{resolved_name}/"))
                })
                .map(|yak| yak.name.clone())
                .collect();

            for child_name in children {
                app.with_yak(&child_name, |yak| yak.update_state("done".to_string()))?;
            }
        } else {
            // Mark just this yak as done/undone
            let new_state = if self.undo { "todo" } else { "done" };
            app.with_yak(&resolved_name, |yak| {
                yak.update_state(new_state.to_string())
            })?;
        }

        Ok(())
    }
}

impl UseCase for DoneYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
