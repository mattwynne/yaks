// File and directory I/O operations for directory-based storage

use super::query::resolve_by_id;
use crate::domain::slug::{slugify, Name, YakId};
use crate::domain::{CONTEXT_FIELD, ID_FIELD, NAME_FIELD};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

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

    Ok(())
}

/// Delete a yak directory and rescue its children.
pub(super) fn delete_yak(base_path: &Path, id: &YakId) -> Result<()> {
    let dir = super::query::yak_dir(base_path, id.as_str());
    if dir.exists() {
        // Before removing, move any child yak directories to root
        // so they survive parent deletion (orphan rescue).
        rescue_children(base_path, &dir)?;
        fs::remove_dir_all(&dir).with_context(|| format!("Failed to remove yak '{id}'"))?;
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
        fs::write(from_dir.join(NAME_FIELD), new_name.as_str())
            .with_context(|| format!("Failed to update name file for '{}'", new_name))?;
        return Ok(());
    }

    if to_dir.exists() {
        anyhow::bail!("Yak '{}' already exists", new_name);
    }

    fs::rename(&from_dir, &to_dir)
        .with_context(|| format!("Failed to rename '{}' to '{}'", id, new_name))?;

    // Update name file to reflect new name
    fs::write(to_dir.join(NAME_FIELD), new_name.as_str())
        .with_context(|| format!("Failed to update name file for '{}'", new_name))?;

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

    fs::rename(&current_dir, &new_dir)
        .with_context(|| format!("Failed to move yak '{}' to new parent", id))?;

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
                    fs::rename(&path, &target).context("Failed to rescue child yak")?;
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
    for entry in fs::read_dir(base_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join(CONTEXT_FIELD).exists() {
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
    let field_path = dir.join(field_name);
    fs::write(&field_path, content)
        .with_context(|| format!("Failed to write field '{field_name}' for '{id}'"))
}

/// Read a field from a yak directory.
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
