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
        // Resolve yak name (exact or fuzzy match) using store
        let resolved_name = {
            let all_yaks = app.store.list_yaks()?;
            let name = &self.name;

            if app.store.yak_exists(name) {
                name.clone()
            } else {
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

        // Collect names to update for recursive mode before entering with_yak_map
        let descendants = if self.recursive && !self.undo {
            let all_yaks = app.store.list_yaks()?;
            let mut names: Vec<String> = all_yaks
                .iter()
                .filter(|yak| {
                    yak.name == resolved_name
                        || yak.name.starts_with(&format!("{resolved_name}/"))
                })
                .map(|yak| yak.name.clone())
                .collect();
            // Sort by depth descending (leaves first) so children are
            // marked done before parents, passing hierarchy validation
            names.sort_by(|a, b| {
                let depth_a = a.matches('/').count();
                let depth_b = b.matches('/').count();
                depth_b.cmp(&depth_a)
            });
            Some(names)
        } else {
            None
        };

        // All state mutations go through YakMap.update_state()
        let new_state = if self.undo { "todo" } else { "done" };

        app.with_yak_map(move |yak_map| {
            if let Some(names) = descendants {
                for name in names {
                    yak_map.update_state(name, new_state.to_string())?;
                }
            } else {
                yak_map.update_state(resolved_name, new_state.to_string())?;
            }
            Ok(())
        })?;

        Ok(())
    }
}

impl UseCase for DoneYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}
