// Use case: Set the state of a yak

use crate::domain::STATE_FIELD;
use anyhow::Result;

use super::Application;

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

    pub fn execute(&self, app: &Application) -> Result<()> {
        // Validate state value
        if !["todo", "wip", "done"].contains(&self.state.as_str()) {
            anyhow::bail!(
                "Invalid state '{}'. Valid states are: todo, wip, done",
                self.state
            );
        }

        // Resolve yak name (exact or fuzzy match)
        let resolved_name = app.storage.find_yak(&self.name)?;

        // Set the state
        app.storage
            .write_field(&resolved_name, STATE_FIELD, &self.state)?;

        // If child state changes from "todo", set all parents to "wip"
        if self.state != "todo" {
            self.set_parents_to_wip(app, &resolved_name)?;
        }

        // Log the command
        app.log
            .log_command(&format!("state {} {}", self.name, self.state))?;

        Ok(())
    }

    fn set_parents_to_wip(&self, app: &Application, yak_name: &str) -> Result<()> {
        let parts: Vec<&str> = yak_name.split('/').collect();
        if parts.len() <= 1 {
            return Ok(()); // No parent
        }

        // Build parent path and set to wip
        for i in 1..parts.len() {
            let parent_path = parts[0..i].join("/");
            if app.storage.get_yak(&parent_path).is_ok() {
                // Check current state of parent - only set to wip if it's currently "todo"
                let parent_yak = app.storage.get_yak(&parent_path)?;
                if parent_yak.state == "todo" {
                    app.storage.write_field(&parent_path, STATE_FIELD, "wip")?;
                }
            }
        }

        Ok(())
    }
}
