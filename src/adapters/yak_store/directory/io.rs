// File and directory I/O operations for directory-based storage

use super::permissions::{
    make_dir_readonly, make_dir_writable_recursive, make_readonly, make_writable,
};
use super::query::resolve_by_id;
use crate::domain::slug::{slugify, Name, YakId};
use crate::domain::{YakBlockerSnapshot, CONTEXT_FIELD, ID_FIELD, NAME_FIELD};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const BLOCKERS_FILE: &str = ".blockers.json";

/// Create a new yak directory.
pub(super) fn create_yak(
    base_path: &Path,
    name: &Name,
    id: &YakId,
    parent_id: Option<&YakId>,
) -> Result<()> {
    // Use slug (from name) as directory name for human readability.
    // Fall back to name directly for backward compat (empty id = legacy).
    let dir_name = if id.as_str().is_empty() {
        name.as_str().to_string()
    } else {
        slugify(name.as_str()).to_string()
    };

    // Determine parent directory: base_path or parent's directory
    let parent_dir = match parent_id {
        Some(pid) => resolve_by_id(base_path, pid.as_str())
            .ok_or_else(|| anyhow::anyhow!("Parent yak '{}' not found", pid))?,
        None => base_path.to_path_buf(),
    };

    let dir = parent_dir.join(&dir_name);
    if dir.join(CONTEXT_FIELD).exists() {
        anyhow::bail!("Yak '{}' already exists", name);
    }

    // Make parent writable before creating the new yak (only if it's a yak directory, not base_path)
    if parent_dir != base_path {
        make_writable(&parent_dir)?;
    }

    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create yak directory: {dir_name}"))?;

    // Create empty context.md file by default
    fs::write(dir.join(CONTEXT_FIELD), "")
        .with_context(|| format!("Failed to create context.md for yak: {name}"))?;

    // Write name file for name→directory resolution
    fs::write(dir.join(NAME_FIELD), name.as_str())
        .with_context(|| format!("Failed to write name file for yak: {name}"))?;

    // Write id file so the immutable ID is stored inside the directory
    if !id.as_str().is_empty() {
        fs::write(dir.join(ID_FIELD), id.as_str())
            .with_context(|| format!("Failed to write id file for yak: {name}"))?;
    }

    // Make the new yak directory readonly recursively
    make_dir_readonly(&dir)?;

    // Make parent readonly again (only if it's a yak directory, not base_path)
    if parent_dir != base_path {
        make_readonly(&parent_dir)?;
    }

    Ok(())
}

/// Delete a yak directory and rescue its children.
pub(super) fn delete_yak(base_path: &Path, id: &YakId) -> Result<()> {
    let dir = super::query::yak_dir(base_path, id.as_str());
    if dir.exists() {
        // Make the yak directory writable recursively
        make_dir_writable_recursive(&dir)?;

        // Make parent dir writable (to remove the entry) - but only if it's a yak directory
        let parent_dir = dir.parent().unwrap_or(base_path);
        if parent_dir != base_path {
            make_writable(parent_dir)?;
        }

        // Before removing, move any child yak directories to root
        // so they survive parent deletion (orphan rescue).
        rescue_children(base_path, &dir)?;

        fs::remove_dir_all(&dir).with_context(|| format!("Failed to remove yak '{id}'"))?;

        // Make parent readonly again - but only if it's a yak directory
        if parent_dir != base_path {
            make_readonly(parent_dir)?;
        }
    }
    Ok(())
}

/// Rename a yak (update its name and possibly move its directory).
pub(super) fn rename_yak(base_path: &Path, id: &YakId, new_name: &Name) -> Result<()> {
    let from_dir = super::query::yak_dir(base_path, id.as_str());

    if !from_dir.exists() {
        anyhow::bail!("yak '{}' not found", id);
    }

    // Compute new slug-based directory name
    let new_slug = slugify(new_name.as_str()).to_string();

    // Target directory is in the same parent as the current directory
    let parent_dir = from_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory for '{}'", id))?;
    let to_dir = parent_dir.join(&new_slug);

    if to_dir == from_dir {
        // Slug unchanged - just update the name file
        make_writable(&from_dir)?;
        let name_file = from_dir.join(NAME_FIELD);
        make_writable(&name_file)?;
        fs::write(&name_file, new_name.as_str())
            .with_context(|| format!("Failed to update name file for '{}'", new_name))?;
        make_readonly(&name_file)?;
        make_readonly(&from_dir)?;
        return Ok(());
    }

    if to_dir.exists() {
        anyhow::bail!("Yak '{}' already exists", new_name);
    }

    // Make parent dir writable (only if it's a yak directory), make yak dir writable recursively
    if parent_dir != base_path {
        make_writable(parent_dir)?;
    }
    make_dir_writable_recursive(&from_dir)?;

    fs::rename(&from_dir, &to_dir)
        .with_context(|| format!("Failed to rename '{}' to '{}'", id, new_name))?;

    // Update name file to reflect new name
    let name_file = to_dir.join(NAME_FIELD);
    fs::write(&name_file, new_name.as_str())
        .with_context(|| format!("Failed to update name file for '{}'", new_name))?;

    // Make the new dir readonly recursively, parent dir readonly (only if it's a yak directory)
    make_dir_readonly(&to_dir)?;
    if parent_dir != base_path {
        make_readonly(parent_dir)?;
    }

    Ok(())
}

/// Move a yak to a new parent.
pub(super) fn reparent_yak(
    base_path: &Path,
    id: &YakId,
    new_parent_id: Option<&YakId>,
) -> Result<()> {
    let current_dir = resolve_by_id(base_path, id.as_str())
        .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

    let new_parent_dir = match new_parent_id {
        Some(pid) => resolve_by_id(base_path, pid.as_str())
            .ok_or_else(|| anyhow::anyhow!("parent yak '{}' not found", pid))?,
        None => base_path.to_path_buf(),
    };

    // Preserve the existing slug-based directory name when moving
    let dir_name = current_dir
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine directory name for '{}'", id))?;
    let new_dir = new_parent_dir.join(dir_name);
    if new_dir.exists() {
        anyhow::bail!("Target location already exists for '{}'", id);
    }

    // Make old parent writable (only if it's a yak directory), new parent writable (only if it's a yak directory), yak dir writable
    let old_parent_dir = current_dir.parent().unwrap_or(base_path);
    if old_parent_dir != base_path {
        make_writable(old_parent_dir)?;
    }
    if new_parent_dir != base_path {
        make_writable(&new_parent_dir)?;
    }
    make_dir_writable_recursive(&current_dir)?;

    fs::rename(&current_dir, &new_dir)
        .with_context(|| format!("Failed to move yak '{}' to new parent", id))?;

    // Make yak dir readonly, both parent dirs readonly (only if they're yak directories)
    make_dir_readonly(&new_dir)?;
    if new_parent_dir != base_path {
        make_readonly(&new_parent_dir)?;
    }
    if old_parent_dir != base_path {
        make_readonly(old_parent_dir)?;
    }

    Ok(())
}

/// Move any immediate child yak directories to the base path (root).
/// Called before deleting a parent so nested children are not lost.
fn rescue_children(base_path: &Path, parent_dir: &Path) -> Result<()> {
    if let Ok(entries) = fs::read_dir(parent_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join(CONTEXT_FIELD).exists() {
                // This is a child yak directory - move to root
                let dir_name = path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Cannot get dir name"))?;
                let target = base_path.join(dir_name);
                if !target.exists() {
                    // Make child dir writable (base_path never needs to be made writable as it stays writable)
                    make_dir_writable_recursive(&path)?;

                    fs::rename(&path, &target).context("Failed to rescue child yak")?;

                    // Make it readonly at new location (base_path stays writable)
                    make_dir_readonly(&target)?;
                }
            }
        }
    }
    Ok(())
}

/// Remove all yak directories from the base path.
/// A directory is a yak if it contains a `context.md` file.
/// Non-yak files (e.g. `.schema-version`) are preserved.
pub(super) fn clear(base_path: &Path) -> Result<()> {
    if !base_path.exists() {
        fs::create_dir_all(base_path)?;
        return Ok(());
    }

    let blockers_file = base_path.join(BLOCKERS_FILE);
    if blockers_file.exists() {
        fs::remove_file(blockers_file)?;
    }

    // base_path stays writable, only make individual yak directories writable before removing
    for entry in fs::read_dir(base_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join(CONTEXT_FIELD).exists() {
            // Make it writable recursively, then remove
            make_dir_writable_recursive(&path)?;
            fs::remove_dir_all(&path)?;
        }
    }

    Ok(())
}

/// Write a field to a yak directory.
pub(super) fn write_field(
    base_path: &Path,
    id: &YakId,
    field_name: &str,
    content: &str,
) -> Result<()> {
    let dir = super::query::yak_dir(base_path, id.as_str());
    if !dir.exists() {
        anyhow::bail!("yak '{}' not found", id);
    }

    // Make the yak dir writable
    make_writable(&dir)?;

    let field_path = dir.join(field_name);

    // Make the target file writable (if it exists)
    make_writable(&field_path)?;

    // Write the file
    fs::write(&field_path, content)
        .with_context(|| format!("Failed to write field '{field_name}' for '{id}'"))?;

    // Make the file readonly
    make_readonly(&field_path)?;

    // Make the dir readonly
    make_readonly(&dir)?;

    Ok(())
}

/// Read a field from a yak directory.
pub(super) fn read_blockers(base_path: &Path) -> Result<Vec<YakBlockerSnapshot>> {
    let path = base_path.join(BLOCKERS_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let json =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let values: Vec<serde_json::Value> = serde_json::from_str(&json)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    let blockers = values
        .into_iter()
        .filter_map(|value| {
            Some(YakBlockerSnapshot {
                target: YakId::from(value.get("target")?.as_str()?),
                blocker: YakId::from(value.get("blocker")?.as_str()?),
                reason: value
                    .get("reason")
                    .and_then(|reason| reason.as_str())
                    .map(str::to_string),
            })
        })
        .collect();
    Ok(blockers)
}

pub(super) fn write_blockers(base_path: &Path, blockers: &[YakBlockerSnapshot]) -> Result<()> {
    fs::create_dir_all(base_path)?;
    let path = base_path.join(BLOCKERS_FILE);
    if blockers.is_empty() {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }

    let values = blockers
        .iter()
        .map(|blocker| {
            serde_json::json!({
                "target": blocker.target.as_str(),
                "blocker": blocker.blocker.as_str(),
                "reason": blocker.reason,
            })
        })
        .collect::<Vec<_>>();
    fs::write(&path, serde_json::to_string(&values)?)
        .with_context(|| format!("Failed to write {}", path.display()))
}

pub(super) fn read_field(base_path: &Path, id: &YakId, field_name: &str) -> Result<String> {
    let dir = resolve_by_id(base_path, id.as_str())
        .or_else(|| {
            let d = super::query::yak_dir(base_path, id.as_str());
            if d.exists() {
                Some(d)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

    let field_path = dir.join(field_name);
    fs::read_to_string(&field_path)
        .with_context(|| format!("Failed to read field '{field_name}' for '{id}'"))
}
