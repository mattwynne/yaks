// Field serialization/deserialization for directory-based storage

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::field::RESERVED_FIELDS;
use crate::domain::slug::YakId;
use crate::domain::{ID_FIELD, NAME_FIELD};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Read the yak ID from a directory's id file, falling back to dir name.
pub(super) fn read_id_from_dir(dir: &Path, fallback: &str) -> YakId {
    fs::read_to_string(dir.join(ID_FIELD))
        .map(|s| YakId::from(s.trim().to_string()))
        .unwrap_or_else(|_| {
            YakId::from(
                dir.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(fallback)
                    .to_string(),
            )
        })
}

/// Read custom fields (non-reserved files) from a yak directory.
pub(super) fn read_custom_fields(dir: &Path) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if !RESERVED_FIELDS.contains(&name) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        fields.insert(name.to_string(), content);
                    }
                }
            }
        }
    }
    fields
}

/// Read direct child yak IDs from subdirectories of a yak directory.
pub(super) fn read_children(dir: &Path) -> Vec<YakId> {
    let mut children = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if !path.join(crate::domain::CONTEXT_FIELD).exists() {
                continue;
            }
            let id = read_id_from_dir(
                &path,
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown"),
            );
            children.push(id);
        }
    }
    children
}

/// Read the parent yak's ID from the filesystem.
/// If the parent directory is also a yak (has context.md), read its id file.
pub(super) fn read_parent_id(dir: &Path, base_path: &Path) -> Option<YakId> {
    dir.parent().and_then(|parent| {
        if parent != base_path && parent.join(crate::domain::CONTEXT_FIELD).exists() {
            let fallback = parent
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            Some(read_id_from_dir(parent, fallback))
        } else {
            None
        }
    })
}

/// Read created_by and created_at from .created.json in a yak directory.
/// Returns (Author::unknown(), Timestamp::zero()) if the file is missing or unparseable.
pub(super) fn read_metadata(dir: &Path) -> (Author, Timestamp) {
    let content = fs::read_to_string(dir.join(".created.json"));
    if let Ok(content) = content {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            let author = Author {
                name: json["created_by"]["name"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                email: json["created_by"]["email"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            };
            let timestamp = Timestamp(json["created_at"].as_i64().unwrap_or(0));
            return (author, timestamp);
        }
    }
    (Author::unknown(), Timestamp::zero())
}

/// Read the leaf name for a yak at the given path.
/// Returns the content of the name file, or falls back to the directory name.
pub(super) fn read_leaf_name(path: &Path) -> String {
    fs::read_to_string(path.join(NAME_FIELD)).unwrap_or_else(|_| {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    })
}
