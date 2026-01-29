use crate::domain::{validate_yak_name, YakState};
use crate::ports::{GitRepository, OutputFormat, OutputFormatter, YakFilter, YakStorage};
use anyhow::{Context, Result};
use std::io::{self, BufRead};

pub struct YakApp<S: YakStorage, G: GitRepository, O: OutputFormatter> {
    storage: S,
    git: G,
    formatter: O,
}

impl<S: YakStorage, G: GitRepository, O: OutputFormatter> YakApp<S, G, O> {
    pub fn new(mut storage: S, git: G, formatter: O) -> Self {
        let _ = storage.migrate_done_to_state();
        Self {
            storage,
            git,
            formatter,
        }
    }

    pub fn check_preconditions(&self) -> Result<()> {
        if !self.git.is_repository() {
            anyhow::bail!("Error: not in a git repository\nyx must be run from within a git repository");
        }

        if !self.git.check_ignore(".yaks")? {
            anyhow::bail!("Error: .yaks folder is not gitignored\nPlease add .yaks to your .gitignore file");
        }

        Ok(())
    }

    pub fn add(&mut self, name: Option<String>) -> Result<()> {
        if let Some(name) = name {
            self.storage.add(&name)?;
            self.git.log_command(&format!("add {name}"))?;
        } else {
            println!("Enter yaks (empty line to finish):");
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                let line = line?;
                if line.trim().is_empty() {
                    break;
                }
                validate_yak_name(&line)?;
                self.storage.add(&line)?;
                self.git.log_command(&format!("add {line}"))?;
            }
        }
        Ok(())
    }

    pub fn list(&self, format: OutputFormat, filter: YakFilter) -> Result<String> {
        let yaks = self.storage.list()?;
        Ok(self.formatter.format_yak_list(&yaks, format, filter))
    }

    pub fn done(&mut self, name: &str, undo: bool, recursive: bool) -> Result<()> {
        let state = if undo {
            YakState::Todo
        } else {
            YakState::Done
        };

        if recursive {
            self.storage.update_state_recursive(name, state)?;
        } else {
            self.storage.update_state(name, state)?;
        }

        let resolved_name = self
            .storage
            .find_yak(name)?
            .unwrap_or_else(|| name.to_string());

        let cmd = if recursive {
            format!("done --recursive {resolved_name}")
        } else if undo {
            format!("done --undo {resolved_name}")
        } else {
            format!("done {resolved_name}")
        };

        self.git.log_command(&cmd)?;
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<()> {
        let resolved_name = self
            .storage
            .find_yak(name)?
            .context("Yak not found")?;

        self.storage.remove(name)?;
        self.git.log_command(&format!("rm {resolved_name}"))?;
        Ok(())
    }

    pub fn prune(&mut self) -> Result<()> {
        let yaks = self.storage.list()?;
        let done_yaks: Vec<String> = yaks
            .iter()
            .filter(|y| y.state == YakState::Done)
            .map(|y| y.name.clone())
            .collect();

        for yak_name in done_yaks {
            // Call remove() method which handles resolution and logging
            self.remove(&yak_name)?;
        }
        Ok(())
    }

    pub fn rename(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        let resolved_old = self
            .storage
            .find_yak(old_name)?
            .context("Yak not found")?;

        self.storage.rename(old_name, new_name)?;
        self.git
            .log_command(&format!("move {resolved_old} {new_name}"))?;
        Ok(())
    }

    pub fn show_context(&self, name: &str) -> Result<String> {
        let resolved_name = self
            .storage
            .find_yak(name)?
            .context(format!("Error: yak '{name}' not found"))?;

        let yak = self
            .storage
            .get(&resolved_name)?
            .context(format!("Error: yak '{name}' not found"))?;

        Ok(self.formatter.format_yak_with_context(&yak))
    }

    pub fn edit_context(&mut self, name: &str, context: Option<String>) -> Result<()> {
        let resolved_name = self
            .storage
            .find_yak(name)?
            .context(format!("Error: yak '{name}' not found"))?;

        if let Some(content) = context {
            self.storage.set_context(&resolved_name, &content)?;
        } else {
            let mut content = String::new();
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                content.push_str(&line?);
                content.push('\n');
            }
            self.storage.set_context(&resolved_name, &content)?;
        }

        self.git.log_command(&format!("context {resolved_name}"))?;
        Ok(())
    }

    pub fn sync(&self) -> Result<()> {
        self.git.sync()
    }

    pub fn completions(&self, cmd: Option<&str>, flag: Option<&str>) -> Result<Vec<String>> {
        let yaks = self.storage.list()?;
        let mut results = Vec::new();

        for yak in yaks {
            let should_include = match (cmd, flag) {
                (Some("done"), Some("--undo")) => yak.state == YakState::Done,
                (Some("done"), _) => yak.state != YakState::Done,
                _ => true,
            };

            if should_include {
                results.push(yak.name);
            }
        }

        Ok(results)
    }
}
