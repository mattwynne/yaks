// Query and resolution operations for directory-based storage

use super::fields::{
    read_children, read_custom_fields, read_id_from_dir, read_leaf_name, read_metadata,
    read_parent_id,
};
use crate::domain::slug::{Name, YakId};
use crate::domain::{
    YakState, YakView, CONTEXT_FIELD, ID_FIELD, NAME_FIELD, STATE_FIELD, TAGS_FIELD,
};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Resolve a yak's directory by name or id.
/// Tries: direct path, resolve by id, resolve by name (in that order).
pub(super) fn yak_dir(base_path: &Path, key: &str) -> PathBuf {
    // Try direct path first (backward compat: dir name == yak name)
    let direct = base_path.join(key);
    if direct.exists() {
        return direct;
    }

    // Try resolve by id (finds nested id-based dirs)
    if let Some(dir) = resolve_by_id(base_path, key) {
        return dir;
    }

    // Try resolve by leaf name (scans name files)
    if let Some(dir) = resolve_by_name(base_path, key) {
        return dir;
    }

    // Fallback to direct path (will fail later with "not found")
    direct
}

/// Find a yak directory by its id, searching recursively.
/// Reads the `id` file inside each yak directory and matches against that.
/// Falls back to directory name matching for backward compat (yaks without id files).
pub(super) fn resolve_by_id(base_path: &Path, id: &str) -> Option<PathBuf> {
    if !base_path.exists() {
        return None;
    }
    let mut fallback: Option<PathBuf> = None;
    for entry in WalkDir::new(base_path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.join(CONTEXT_FIELD).exists() {
            continue;
        }
        // Primary: match against id file contents
        let id_file = path.join(ID_FIELD);
        if id_file.exists() {
            if let Ok(stored_id) = fs::read_to_string(&id_file) {
                if stored_id.trim() == id {
                    return Some(path.to_path_buf());
                }
            }
        }
        // Fallback: match against directory name (backward compat)
        if fallback.is_none() && path.file_name().and_then(|n| n.to_str()) == Some(id) {
            fallback = Some(path.to_path_buf());
        }
    }
    fallback
}

/// Scan directories recursively for one whose name file matches the given name.
pub(super) fn resolve_by_name(base_path: &Path, name: &str) -> Option<PathBuf> {
    if !base_path.exists() {
        return None;
    }
    for entry in WalkDir::new(base_path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let name_file = path.join(NAME_FIELD);
        if name_file.exists() {
            if let Ok(stored_name) = fs::read_to_string(&name_file) {
                if stored_name == name {
                    return Some(path.to_path_buf());
                }
            }
        }
    }
    None
}

/// Get a single yak by its ID.
pub(super) fn get_yak(base_path: &Path, id: &YakId) -> Result<YakView> {
    let dir = resolve_by_id(base_path, id.as_str())
        .or_else(|| {
            // Fallback: try yak_dir resolution for backward compat
            let d = yak_dir(base_path, id.as_str());
            if d.exists() && d.join(CONTEXT_FIELD).exists() {
                Some(d)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

    let display_name = read_leaf_name(&dir);

    let context = fs::read_to_string(dir.join(CONTEXT_FIELD))
        .ok()
        .and_then(|c| if c.is_empty() { None } else { Some(c) });

    let state: YakState = fs::read_to_string(dir.join(STATE_FIELD))
        .unwrap_or_else(|_| "todo".to_string())
        .trim()
        .parse()
        .unwrap_or(YakState::Todo);

    let fields = read_custom_fields(&dir);
    let tags: Vec<String> = fs::read_to_string(dir.join(TAGS_FIELD))
        .ok()
        .map(|t| {
            t.lines()
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let children = read_children(&dir);
    let parent_id = read_parent_id(&dir, base_path);
    let (created_by, created_at) = read_metadata(&dir);

    Ok(YakView {
        id: id.clone(),
        name: Name::from(display_name),
        parent_id,
        state,
        context,
        fields,
        tags,
        children,
        created_by,
        created_at,
    })
}

/// List all yaks in the storage.
pub(super) fn list_yaks(base_path: &Path) -> Result<Vec<YakView>> {
    let mut yaks = Vec::new();

    if !base_path.exists() {
        return Ok(yaks);
    }

    // Use WalkDir to recursively find all directories that are yaks
    for entry in WalkDir::new(base_path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let entry = entry?;
        let path = entry.path();

        // Only process directories that have a context.md (are actual yaks)
        if !path.join(CONTEXT_FIELD).exists() {
            continue;
        }

        // Build hierarchical name from directory structure and leaf name files
        let display_name = read_leaf_name(path);

        // Read id from id file, fall back to directory name (backward compat)
        let id = fs::read_to_string(path.join(ID_FIELD))
            .unwrap_or_else(|_| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&display_name)
                    .to_string()
            })
            .trim()
            .to_string();

        let context = fs::read_to_string(path.join(CONTEXT_FIELD))
            .ok()
            .and_then(|c| if c.is_empty() { None } else { Some(c) });

        let state: YakState = fs::read_to_string(path.join(STATE_FIELD))
            .unwrap_or_else(|_| "todo".to_string())
            .trim()
            .parse()
            .unwrap_or(YakState::Todo);

        let fields = read_custom_fields(path);
        let tags: Vec<String> = fs::read_to_string(path.join(TAGS_FIELD))
            .ok()
            .map(|t| {
                t.lines()
                    .filter(|l| !l.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let children = read_children(path);
        let parent_id = read_parent_id(path, base_path);
        let (created_by, created_at) = read_metadata(path);

        yaks.push(YakView {
            id: YakId::from(id),
            name: Name::from(display_name),
            parent_id,
            state,
            context,
            fields,
            tags,
            children,
            created_by,
            created_at,
        });
    }

    Ok(yaks)
}

/// Fuzzy find a yak ID by query string.
pub(super) fn fuzzy_find_yak_id(base_path: &Path, query: &str) -> Result<YakId> {
    // First, try exact match via resolution (handles both old and new format)
    let dir = yak_dir(base_path, query);
    if dir.exists() && dir.join(CONTEXT_FIELD).exists() {
        let id = read_id_from_dir(&dir, query);
        return Ok(id);
    }

    // If not found, try fuzzy match on the name
    let yaks = list_yaks(base_path)?;
    let matches: Vec<&YakView> = yaks
        .iter()
        .filter(|yak| {
            yak.name
                .as_str()
                .to_lowercase()
                .contains(&query.to_lowercase())
        })
        .collect();

    match matches.len() {
        0 => anyhow::bail!("yak '{query}' not found"),
        1 => Ok(matches[0].id.clone()),
        _ => anyhow::bail!("yak name '{query}' is ambiguous"),
    }
}
