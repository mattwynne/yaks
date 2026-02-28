//! Git tree serialization and deserialization for yak data.
//!
//! This module handles building git tree objects from domain events
//! and reading yak snapshots back from git trees.

use anyhow::Result;
use git2::Repository;

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::YakEvent;

/// Builds a git tree object representing a single yak's subtree.
///
/// A yak subtree contains blobs for each field (state, context.md, name,
/// etc.) plus optional metadata. This builder provides a single place to
/// construct these subtrees, used by `build_tree_from_event` (for
/// the `Added` event).
///
/// # Example
///
/// ```ignore
/// let oid = YakSubtreeBuilder::new(&repo)
///     .name("fix the tests")
///     .state("todo")
///     .context("")
///     .metadata(&author, timestamp)
///     .parent_id(Some("parent-a1b2"))
///     .build()?;
/// ```
pub(super) struct YakSubtreeBuilder<'r> {
    repo: &'r Repository,
    entries: Vec<(&'static str, String)>,
    custom_fields: Vec<(String, String)>,
}

impl<'r> YakSubtreeBuilder<'r> {
    pub(super) fn new(repo: &'r Repository) -> Self {
        Self {
            repo,
            entries: Vec::new(),
            custom_fields: Vec::new(),
        }
    }

    /// Set the yak's display name.
    pub(super) fn name(mut self, name: &str) -> Self {
        self.entries.push((".name", name.to_string()));
        self
    }

    /// Set the yak's state (todo, wip, done).
    pub(super) fn state(mut self, state: &str) -> Self {
        self.entries.push((".state", state.to_string()));
        self
    }

    /// Set the yak's context markdown content.
    pub(super) fn context(mut self, content: &str) -> Self {
        self.entries.push((".context.md", content.to_string()));
        self
    }

    /// Set the parent yak's ID, if this yak is nested.
    pub(super) fn parent_id(mut self, parent_id: Option<&str>) -> Self {
        if let Some(pid) = parent_id {
            self.entries.push((".parent_id", pid.to_string()));
        }
        self
    }

    /// Write the `.created.json` blob with author and timestamp.
    pub(super) fn metadata(mut self, author: &Author, timestamp: Timestamp) -> Self {
        let json = serde_json::json!({
            "created_by": {
                "name": author.name,
                "email": author.email
            },
            "created_at": timestamp.as_epoch_secs()
        });
        self.entries.push((".created.json", json.to_string()));
        self
    }

    /// Add custom (non-reserved) fields to the subtree.
    pub(super) fn custom_fields(
        mut self,
        fields: &std::collections::HashMap<String, String>,
    ) -> Self {
        for (name, content) in fields {
            self.custom_fields.push((name.clone(), content.clone()));
        }
        self
    }

    /// Write all collected entries to a new git tree object.
    pub(super) fn build(self) -> Result<git2::Oid> {
        let mut builder = self.repo.treebuilder(None)?;

        for (name, content) in &self.entries {
            let blob = self.repo.blob(content.as_bytes())?;
            builder.insert(name, blob, 0o100644)?;
        }

        for (name, content) in &self.custom_fields {
            let blob = self.repo.blob(content.as_bytes())?;
            builder.insert(name, blob, 0o100644)?;
        }

        Ok(builder.write()?)
    }
}

/// Get a yak's subtree from the root tree by its ID (direct root lookup).
pub(super) fn get_yak_subtree<'r>(
    repo: &'r Repository,
    root: Option<&git2::Tree>,
    yak_id: &str,
) -> Result<Option<git2::Tree<'r>>> {
    let Some(root) = root else {
        return Ok(None);
    };

    match root.get_name(yak_id) {
        Some(entry) => Ok(Some(repo.find_tree(entry.id())?)),
        None => Ok(None),
    }
}

/// Update a file in a yak's subtree, returning new root tree OID.
pub(super) fn update_yak_file(
    repo: &Repository,
    current_tree: Option<&git2::Tree>,
    yak_id: &str,
    file_name: &str,
    content: &str,
) -> Result<git2::Oid> {
    let blob_oid = repo.blob(content.as_bytes())?;

    // Build the yak's subtree
    let yak_subtree = get_yak_subtree(repo, current_tree, yak_id)?;
    let mut yak_builder = repo.treebuilder(yak_subtree.as_ref())?;
    yak_builder.insert(file_name, blob_oid, 0o100644)?;
    let yak_tree_oid = yak_builder.write()?;

    // Rebuild root tree with updated yak subtree
    set_yak_in_root(repo, current_tree, yak_id, Some(yak_tree_oid))
}

/// Set (or remove) a yak subtree in the root tree.
pub(super) fn set_yak_in_root(
    repo: &Repository,
    root: Option<&git2::Tree>,
    yak_id: &str,
    subtree_oid: Option<git2::Oid>,
) -> Result<git2::Oid> {
    let mut builder = repo.treebuilder(root)?;
    match subtree_oid {
        Some(oid) => {
            builder.insert(yak_id, oid, 0o040000)?;
        }
        None => {
            let _ = builder.remove(yak_id);
        }
    }
    Ok(builder.write()?)
}

/// Build an updated tree by applying an event to the current tree.
/// All operations happen in git's object database - no filesystem IO.
pub(super) fn build_tree_from_event(
    repo: &Repository,
    event: &YakEvent,
    current_tree: Option<&git2::Tree>,
) -> Result<git2::Oid> {
    match event {
        YakEvent::Added(e, metadata) => {
            let yak_tree_oid = YakSubtreeBuilder::new(repo)
                .name(e.name.as_str())
                .state("todo")
                .context("")
                .metadata(&metadata.author, metadata.timestamp)
                .parent_id(e.parent_id.as_ref().map(|p| p.as_str()))
                .build()?;
            set_yak_in_root(repo, current_tree, e.id.as_str(), Some(yak_tree_oid))
        }

        YakEvent::Removed(e, _) => {
            // Flat: yak is always at root by its ID
            set_yak_in_root(repo, current_tree, e.id.as_str(), None)
        }

        YakEvent::Moved(e, _) => {
            // In flat structure, moving just updates the parent_id blob
            let yak_id = e.id.as_str();
            let subtree = get_yak_subtree(repo, current_tree, yak_id)?;
            let mut builder = repo.treebuilder(subtree.as_ref())?;

            match &e.new_parent {
                Some(parent_id) => {
                    let blob = repo.blob(parent_id.as_str().as_bytes())?;
                    builder.insert(".parent_id", blob, 0o100644)?;
                }
                None => {
                    let _ = builder.remove(".parent_id");
                }
            }

            let new_subtree_oid = builder.write()?;
            set_yak_in_root(repo, current_tree, yak_id, Some(new_subtree_oid))
        }

        YakEvent::FieldUpdated(e, _) => {
            // Flat: yak is always at root by its ID
            update_yak_file(repo, current_tree, e.id.as_str(), &e.field_name, &e.content)
        }

        YakEvent::Compacted(snapshots, _) => {
            if snapshots.is_empty() {
                // Legacy: no snapshots, preserve current tree
                match current_tree {
                    Some(tree) => Ok(tree.id()),
                    None => {
                        anyhow::bail!("Cannot compact: no tree state exists")
                    }
                }
            } else {
                // Build tree from snapshots
                use super::migration::CURRENT_SCHEMA_VERSION;
                let mut root_builder = repo.treebuilder(None)?;
                for snap in snapshots {
                    let yak_tree_oid = YakSubtreeBuilder::new(repo)
                        .name(snap.name.as_str())
                        .state(&snap.state)
                        .context(snap.context.as_deref().unwrap_or(""))
                        .parent_id(snap.parent_id.as_ref().map(|p| p.as_str()))
                        .metadata(&snap.created_by, snap.created_at)
                        .custom_fields(&snap.fields)
                        .build()?;
                    root_builder.insert(snap.id.as_str(), yak_tree_oid, 0o040000)?;
                }
                let version_blob = repo.blob(CURRENT_SCHEMA_VERSION.to_string().as_bytes())?;
                root_builder.insert(".schema-version", version_blob, 0o100644)?;
                Ok(root_builder.write()?)
            }
        }
    }
}

/// Read the git tree into `Vec<YakSnapshot>`, preserving existing yak IDs.
#[allow(clippy::cognitive_complexity)]
pub(super) fn read_snapshots_from_tree(
    repo: &Repository,
    tree: &git2::Tree,
) -> Result<Vec<crate::domain::yak_snapshot::YakSnapshot>> {
    use crate::domain::field::RESERVED_FIELDS;
    use crate::domain::slug::{Name, YakId};
    use crate::domain::yak_snapshot::YakSnapshot;
    use std::collections::{HashMap, HashSet};

    struct YakData {
        id: String,
        name_str: String,
        subtree_id: git2::Oid,
        parent_id_str: Option<String>,
    }

    let mut yak_data: Vec<YakData> = Vec::new();

    for entry in tree.iter() {
        if entry.kind() != Some(git2::ObjectType::Tree) {
            continue;
        }
        let entry_name = match entry.name() {
            Some(n) => n.to_string(),
            None => continue,
        };

        let subtree = repo.find_tree(entry.id())?;

        let is_yak =
            subtree.get_name(".state").is_some() || subtree.get_name(".context.md").is_some();
        if !is_yak {
            continue;
        }

        let name_str = if let Some(name_entry) = subtree.get_name(".name") {
            let name_blob = repo.find_blob(name_entry.id())?;
            std::str::from_utf8(name_blob.content())?.trim().to_string()
        } else {
            entry_name.clone()
        };

        let parent_id_str = if let Some(pid_entry) = subtree.get_name(".parent_id") {
            let pid_blob = repo.find_blob(pid_entry.id())?;
            Some(std::str::from_utf8(pid_blob.content())?.trim().to_string())
        } else {
            None
        };

        yak_data.push(YakData {
            id: entry_name,
            name_str,
            subtree_id: entry.id(),
            parent_id_str,
        });
    }

    // Topological sort: parents before children
    let mut emitted: HashSet<String> = HashSet::new();
    let mut remaining = yak_data;
    let mut ordered: Vec<YakData> = Vec::new();

    loop {
        let before = remaining.len();
        let mut still_remaining = Vec::new();

        for item in remaining {
            let can_emit = match &item.parent_id_str {
                None => true,
                Some(pid) => emitted.contains(pid),
            };
            if can_emit {
                emitted.insert(item.id.clone());
                ordered.push(item);
            } else {
                still_remaining.push(item);
            }
        }

        remaining = still_remaining;
        if remaining.is_empty() || remaining.len() == before {
            ordered.extend(remaining);
            break;
        }
    }

    let mut snapshots = Vec::new();

    for data in &ordered {
        let subtree = repo.find_tree(data.subtree_id)?;

        // Read .created.json if present
        let meta_oid = subtree.get_name(".created.json").map(|e| e.id());
        let (created_by, created_at) = if let Some(meta_id) = meta_oid {
            if let Ok(meta_blob) = repo.find_blob(meta_id) {
                if let Ok(content) = std::str::from_utf8(meta_blob.content()) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                        use crate::domain::event_metadata::{Author, Timestamp};
                        (
                            Author {
                                name: json["created_by"]["name"]
                                    .as_str()
                                    .unwrap_or("unknown")
                                    .to_string(),
                                email: json["created_by"]["email"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                            },
                            Timestamp(json["created_at"].as_i64().unwrap_or(0)),
                        )
                    } else {
                        (
                            crate::domain::event_metadata::Author::unknown(),
                            crate::domain::event_metadata::Timestamp::zero(),
                        )
                    }
                } else {
                    (
                        crate::domain::event_metadata::Author::unknown(),
                        crate::domain::event_metadata::Timestamp::zero(),
                    )
                }
            } else {
                (
                    crate::domain::event_metadata::Author::unknown(),
                    crate::domain::event_metadata::Timestamp::zero(),
                )
            }
        } else {
            (
                crate::domain::event_metadata::Author::unknown(),
                crate::domain::event_metadata::Timestamp::zero(),
            )
        };

        // State
        let state = if let Some(state_entry) = subtree.get_name(".state") {
            let state_blob = repo.find_blob(state_entry.id())?;
            std::str::from_utf8(state_blob.content())?
                .trim()
                .to_string()
        } else {
            "todo".to_string()
        };

        // Context
        let context = if let Some(context_entry) = subtree.get_name(".context.md") {
            let context_blob = repo.find_blob(context_entry.id())?;
            let content = std::str::from_utf8(context_blob.content())?;
            if content.is_empty() {
                None
            } else {
                Some(content.to_string())
            }
        } else {
            None
        };

        // Custom fields
        let mut fields = HashMap::new();
        for field_entry in subtree.iter() {
            if field_entry.kind() != Some(git2::ObjectType::Blob) {
                continue;
            }
            let field_name = match field_entry.name() {
                Some(n) => n,
                None => continue,
            };
            if RESERVED_FIELDS.contains(&field_name) {
                continue;
            }
            let field_blob = repo.find_blob(field_entry.id())?;
            let content = std::str::from_utf8(field_blob.content())?;
            fields.insert(field_name.to_string(), content.to_string());
        }

        snapshots.push(YakSnapshot {
            id: YakId::from(data.id.as_str()),
            name: Name::from(data.name_str.as_str()),
            parent_id: data.parent_id_str.as_ref().map(|p| YakId::from(p.as_str())),
            state,
            context,
            fields,
            created_by,
            created_at,
        });
    }

    Ok(snapshots)
}
