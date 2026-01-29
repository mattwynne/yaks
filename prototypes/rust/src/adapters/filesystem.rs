use crate::domain::{validate_yak_name, Yak, YakState};
use crate::ports::YakStorage;
use anyhow::{anyhow, bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FilesystemStorage {
    base_path: PathBuf,
}

impl FilesystemStorage {
    #[must_use]
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn yak_path(&self, name: &str) -> PathBuf {
        self.base_path.join(name)
    }

    fn state_file(&self, name: &str) -> PathBuf {
        self.yak_path(name).join("state")
    }

    fn context_file(&self, name: &str) -> PathBuf {
        self.yak_path(name).join("context.md")
    }

    fn read_state(yak_path: &Path) -> YakState {
        let state_file = yak_path.join("state");
        if let Ok(content) = fs::read_to_string(state_file) {
            content.parse().unwrap_or(YakState::Todo)
        } else {
            YakState::Todo
        }
    }

    fn read_context(yak_path: &Path) -> String {
        let context_file = yak_path.join("context.md");
        fs::read_to_string(context_file).unwrap_or_default()
    }

    fn get_mtime(path: &Path) -> std::time::SystemTime {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    }

    fn ensure_parent_yaks(&mut self, name: &str) -> Result<()> {
        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() <= 1 {
            return Ok(());
        }

        for i in 0..parts.len() - 1 {
            let parent_name = parts[..=i].join("/");
            let parent_path = self.yak_path(&parent_name);
            if !parent_path.exists() {
                fs::create_dir_all(&parent_path)?;
                fs::write(self.state_file(&parent_name), "todo")?;
                fs::write(self.context_file(&parent_name), "")?;
            }
        }
        Ok(())
    }

    fn try_exact_match(&self, search_term: &str) -> Option<String> {
        if self.yak_path(search_term).exists() {
            Some(search_term.to_string())
        } else {
            None
        }
    }

    fn try_fuzzy_match(&self, search_term: &str) -> Result<Option<String>> {
        let yaks = self.list()?;
        let matches: Vec<String> = yaks
            .iter()
            .filter(|y| y.name.contains(search_term))
            .map(|y| y.name.clone())
            .collect();

        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches[0].clone())),
            _ => bail!("Error: yak name '{search_term}' is ambiguous"),
        }
    }

    fn has_incomplete_children(&self, name: &str) -> Result<bool> {
        let yak_path = self.yak_path(name);
        if !yak_path.exists() {
            return Ok(false);
        }

        for entry in WalkDir::new(&yak_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_dir())
        {
            let entry = entry?;
            let state = Self::read_state(entry.path());
            if state != YakState::Done {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn update_state_recursively(&mut self, name: &str, state: YakState) -> Result<()> {
        let yak_path = self.yak_path(name);

        // Update this yak's state
        fs::write(self.state_file(name), state.as_str())?;

        // Recursively update all children
        for entry in WalkDir::new(&yak_path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_dir())
        {
            let entry = entry?;
            let child_name = entry.path()
                .strip_prefix(&self.base_path)?
                .to_string_lossy()
                .to_string();
            self.update_state_recursively(&child_name, state)?;
        }

        Ok(())
    }

    fn migrate_done_file(yak_path: &Path) -> Result<()> {
        let done_file = yak_path.join("done");
        if done_file.exists() {
            fs::write(yak_path.join("state"), "done")?;
            fs::remove_file(done_file)?;
        }
        Ok(())
    }
}

impl YakStorage for FilesystemStorage {
    fn add(&mut self, name: &str) -> Result<()> {
        validate_yak_name(name)?;
        self.ensure_parent_yaks(name)?;

        let yak_path = self.yak_path(name);
        fs::create_dir_all(&yak_path)
            .context(format!("Failed to create directory for yak '{name}'"))?;

        fs::write(self.state_file(name), "todo")?;
        fs::write(self.context_file(name), "")?;

        Ok(())
    }

    fn list(&self) -> Result<Vec<Yak>> {
        if !self.base_path.exists() {
            return Ok(Vec::new());
        }

        let mut yaks = Vec::new();

        for entry in WalkDir::new(&self.base_path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| {
                e.file_type().is_dir() && !e.file_name().to_string_lossy().starts_with('.')
            })
        {
            let entry = entry?;
            let path = entry.path();
            let name = path
                .strip_prefix(&self.base_path)?
                .to_string_lossy()
                .to_string();

            let state = Self::read_state(path);
            let context = Self::read_context(path);
            let mtime = Self::get_mtime(path);

            yaks.push(
                Yak::new(name)
                    .with_state(state)
                    .with_context(context)
                    .with_mtime(mtime),
            );
        }

        Ok(yaks)
    }

    fn get(&self, name: &str) -> Result<Option<Yak>> {
        let yak_path = self.yak_path(name);
        if !yak_path.exists() {
            return Ok(None);
        }

        let state = Self::read_state(&yak_path);
        let context = Self::read_context(&yak_path);
        let mtime = Self::get_mtime(&yak_path);

        Ok(Some(
            Yak::new(name.to_string())
                .with_state(state)
                .with_context(context)
                .with_mtime(mtime),
        ))
    }

    fn update_state(&mut self, name: &str, state: YakState) -> Result<()> {
        let resolved_name = self
            .find_yak(name)?
            .ok_or_else(|| anyhow!("Error: yak '{name}' not found"))?;

        if state == YakState::Done && self.has_incomplete_children(&resolved_name)? {
            bail!(
                "Error: cannot mark '{resolved_name}' as done - it has incomplete children"
            );
        }

        fs::write(self.state_file(&resolved_name), state.as_str())?;
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<()> {
        // For direct removal (like prune), use name as-is if it's a path
        let yak_path = self.yak_path(name);
        if yak_path.exists() {
            fs::remove_dir_all(yak_path)?;
            return Ok(());
        }

        // Otherwise resolve it
        let resolved_name = self
            .find_yak(name)?
            .ok_or_else(|| anyhow!("Error: yak '{name}' not found"))?;

        let yak_path = self.yak_path(&resolved_name);
        fs::remove_dir_all(yak_path)?;
        Ok(())
    }

    fn set_context(&mut self, name: &str, context: &str) -> Result<()> {
        let resolved_name = self
            .find_yak(name)?
            .ok_or_else(|| anyhow!("Error: yak '{name}' not found"))?;

        fs::write(self.context_file(&resolved_name), context)?;
        Ok(())
    }

    fn get_context(&self, name: &str) -> Result<String> {
        let resolved_name = self
            .find_yak(name)?
            .ok_or_else(|| anyhow!("Error: yak '{name}' not found"))?;

        Ok(Self::read_context(&self.yak_path(&resolved_name)))
    }

    fn rename(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        let resolved_old = self
            .find_yak(old_name)?
            .ok_or_else(|| anyhow!("Error: yak '{old_name}' not found"))?;

        validate_yak_name(new_name)?;
        self.ensure_parent_yaks(new_name)?;

        let old_path = self.yak_path(&resolved_old);
        let new_path = self.yak_path(new_name);

        fs::rename(old_path, new_path)?;
        Ok(())
    }

    fn find_yak(&self, search_term: &str) -> Result<Option<String>> {
        if let Some(exact) = self.try_exact_match(search_term) {
            return Ok(Some(exact));
        }
        self.try_fuzzy_match(search_term)
    }

    fn update_state_recursive(&mut self, name: &str, state: YakState) -> Result<()> {
        let resolved_name = self
            .find_yak(name)?
            .ok_or_else(|| anyhow!("Error: yak '{name}' not found"))?;

        self.update_state_recursively(&resolved_name, state)
    }

    fn migrate_done_to_state(&mut self) -> Result<()> {
        if !self.base_path.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(&self.base_path)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| e.file_type().is_dir())
        {
            let entry = entry?;
            Self::migrate_done_file(entry.path())?;
        }

        Ok(())
    }
}
