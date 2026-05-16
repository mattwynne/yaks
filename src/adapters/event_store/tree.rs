//! Git tree serialization and deserialization for yak data.
//!
//! This module handles building git tree objects from domain events
//! and reading yak snapshots back from git trees.

use anyhow::Result;
use git2::Repository;

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::{Yak, YakEvent};

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

    /// Set the yak's state (todo, wip, blocked, done).
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

    /// Set the yak's tags (stored as newline-separated blob).
    /// Only writes the .tags blob if the vec is non-empty.
    pub(super) fn tags(mut self, tags: &[String]) -> Self {
        if !tags.is_empty() {
            let content = tags.join("\n");
            self.entries.push((".tags", content));
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

fn apply_moved_event(
    repo: &Repository,
    current_tree: Option<&git2::Tree>,
    e: &crate::domain::events::MovedEvent,
) -> Result<git2::Oid> {
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

fn preserve_current_or_empty_tree(
    repo: &Repository,
    current_tree: Option<&git2::Tree>,
) -> Result<git2::Oid> {
    match current_tree {
        Some(tree) => Ok(tree.id()),
        None => {
            let builder = repo.treebuilder(None)?;
            Ok(builder.write()?)
        }
    }
}

fn apply_compacted_event(
    repo: &Repository,
    current_tree: Option<&git2::Tree>,
    snapshots: &[Yak],
    removed_yak_ids: &[crate::domain::slug::YakId],
) -> Result<git2::Oid> {
    if snapshots.is_empty() {
        // Legacy: no snapshots, preserve current tree
        return match current_tree {
            Some(tree) => Ok(tree.id()),
            None => anyhow::bail!("Cannot compact: no tree state exists"),
        };
    }

    // Build tree from snapshots
    use super::migration::CURRENT_SCHEMA_VERSION;
    let mut root_builder = repo.treebuilder(None)?;
    for snap in snapshots {
        let yak_tree_oid = YakSubtreeBuilder::new(repo)
            .name(snap.name.as_str())
            .state(&snap.state.to_string())
            .context(snap.context.as_deref().unwrap_or(""))
            .parent_id(snap.parent_id.as_ref().map(|p| p.as_str()))
            .metadata(&snap.created_by, snap.created_at)
            .custom_fields(&snap.fields)
            .tags(&snap.tags)
            .build()?;
        root_builder.insert(snap.id.as_str(), yak_tree_oid, 0o040000)?;
    }
    let version_blob = repo.blob(CURRENT_SCHEMA_VERSION.to_string().as_bytes())?;
    root_builder.insert(".schema-version", version_blob, 0o100644)?;

    // Store removed yak IDs
    if !removed_yak_ids.is_empty() {
        let removed_ids_content = removed_yak_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let removed_blob = repo.blob(removed_ids_content.as_bytes())?;
        root_builder.insert(".removed-yaks", removed_blob, 0o100644)?;
    }

    Ok(root_builder.write()?)
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
                .tags(&[])
                .build()?;
            set_yak_in_root(repo, current_tree, e.id.as_str(), Some(yak_tree_oid))
        }

        YakEvent::Removed(e, _) => {
            // Flat: yak is always at root by its ID
            set_yak_in_root(repo, current_tree, e.id.as_str(), None)
        }

        YakEvent::Moved(e, _) => apply_moved_event(repo, current_tree, e),

        YakEvent::FieldUpdated(e, _) => {
            // Flat: yak is always at root by its ID
            update_yak_file(repo, current_tree, e.id.as_str(), &e.field_name, &e.content)
        }

        YakEvent::BlockerAdded(_, _)
        | YakEvent::BlockerUpdated(_, _)
        | YakEvent::BlockerRemoved(_, _) => preserve_current_or_empty_tree(repo, current_tree),

        YakEvent::Compacted(snapshots, removed_yak_ids, _)
        | YakEvent::Migrated(snapshots, removed_yak_ids, _) => {
            apply_compacted_event(repo, current_tree, snapshots, removed_yak_ids)
        }
    }
}

/// Read a blob entry from a subtree as a trimmed string.
fn read_blob_str(repo: &Repository, subtree: &git2::Tree, name: &str) -> Result<Option<String>> {
    match subtree.get_name(name) {
        Some(entry) => {
            let blob = repo.find_blob(entry.id())?;
            Ok(Some(
                std::str::from_utf8(blob.content())?.trim().to_string(),
            ))
        }
        None => Ok(None),
    }
}

/// Parse `.created.json` metadata from a yak subtree, returning defaults on any error.
fn read_created_metadata(repo: &Repository, subtree: &git2::Tree) -> (Author, Timestamp) {
    let parsed = (|| -> Option<(Author, Timestamp)> {
        let entry = subtree.get_name(".created.json")?;
        let blob = repo.find_blob(entry.id()).ok()?;
        let content = std::str::from_utf8(blob.content()).ok()?;
        let json: serde_json::Value = serde_json::from_str(content).ok()?;
        Some((
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
        ))
    })();
    parsed.unwrap_or_else(|| (Author::unknown(), Timestamp::zero()))
}

/// Read custom (non-reserved) fields from a yak subtree.
fn read_custom_fields(
    repo: &Repository,
    subtree: &git2::Tree,
) -> Result<std::collections::HashMap<String, String>> {
    use crate::domain::field::RESERVED_FIELDS;
    let mut fields = std::collections::HashMap::new();
    for entry in subtree.iter() {
        if entry.kind() != Some(git2::ObjectType::Blob) {
            continue;
        }
        let name = match entry.name() {
            Some(n) => n,
            None => continue,
        };
        if RESERVED_FIELDS.contains(&name) {
            continue;
        }
        let blob = repo.find_blob(entry.id())?;
        let content = std::str::from_utf8(blob.content())?;
        fields.insert(name.to_string(), content.to_string());
    }
    Ok(fields)
}

struct YakData {
    id: String,
    name_str: String,
    subtree_id: git2::Oid,
    parent_id_str: Option<String>,
}

/// Collect raw yak data from the root tree entries.
fn collect_yak_entries(repo: &Repository, tree: &git2::Tree) -> Result<Vec<YakData>> {
    let mut yak_data = Vec::new();
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
        let name_str =
            read_blob_str(repo, &subtree, ".name")?.unwrap_or_else(|| entry_name.clone());
        let parent_id_str = read_blob_str(repo, &subtree, ".parent_id")?;
        yak_data.push(YakData {
            id: entry_name,
            name_str,
            subtree_id: entry.id(),
            parent_id_str,
        });
    }
    Ok(yak_data)
}

/// Topological sort: emit parents before children, append orphans at end.
fn topological_sort(yak_data: Vec<YakData>) -> Vec<YakData> {
    use std::collections::HashSet;
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
    ordered
}

/// Read the git tree into `Vec<Yak>`, preserving existing yak IDs.
pub(super) fn read_snapshots_from_tree(
    repo: &Repository,
    tree: &git2::Tree,
) -> Result<Vec<crate::domain::Yak>> {
    use crate::domain::slug::{Name, YakId};
    use crate::domain::{Yak, YakState};

    let yak_data = collect_yak_entries(repo, tree)?;
    let ordered = topological_sort(yak_data);

    let mut snapshots = Vec::new();
    for data in &ordered {
        let subtree = repo.find_tree(data.subtree_id)?;
        let (created_by, created_at) = read_created_metadata(repo, &subtree);

        let state: YakState = read_blob_str(repo, &subtree, ".state")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(YakState::Todo);

        let context = read_blob_str(repo, &subtree, ".context.md")?.filter(|s| !s.is_empty());

        let fields = read_custom_fields(repo, &subtree)?;

        // Read tags from .tags blob (newline-separated)
        let tags = read_blob_str(repo, &subtree, ".tags")?
            .map(|s| {
                s.lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .map(|line| line.to_string())
                    .collect()
            })
            .unwrap_or_default();

        snapshots.push(Yak {
            id: YakId::from(data.id.as_str()),
            name: Name::from(data.name_str.as_str()),
            parent_id: data.parent_id_str.as_ref().map(|p| YakId::from(p.as_str())),
            state,
            context,
            fields,
            tags,
            created_by,
            created_at,
        });
    }

    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::{Author, Timestamp};
    use crate::domain::slug::{Name, YakId};
    use crate::domain::{Yak, YakState};
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, Repository) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        (tmp, repo)
    }

    #[test]
    fn tags_survive_compaction_roundtrip() {
        // Create a test git repo
        let (_tmp, repo) = setup_test_repo();

        // Create a yak with tags
        let yak = Yak {
            id: YakId::from("test-yak-a1b2"),
            name: Name::from("test yak"),
            parent_id: None,
            state: YakState::Wip,
            context: Some("some context".to_string()),
            fields: std::collections::HashMap::new(),
            tags: vec!["urgent".to_string(), "backend".to_string()],
            created_by: Author {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
            },
            created_at: Timestamp(1234567890),
        };

        // Create a Compacted event with the yak
        let event = crate::domain::YakEvent::Compacted(
            vec![yak.clone()],
            vec![],
            crate::domain::event_metadata::EventMetadata::default_legacy(),
        );

        // Build tree from the compacted event
        let tree_oid = build_tree_from_event(&repo, &event, None).unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        // Read snapshots back from the tree
        let snapshots = read_snapshots_from_tree(&repo, &tree).unwrap();

        // Verify tags survived
        assert_eq!(snapshots.len(), 1);
        let restored_yak = &snapshots[0];
        assert_eq!(restored_yak.id.as_str(), "test-yak-a1b2");
        assert_eq!(restored_yak.tags.len(), 2);
        assert_eq!(restored_yak.tags[0], "urgent");
        assert_eq!(restored_yak.tags[1], "backend");
    }

    #[test]
    fn empty_tags_not_written_to_tree() {
        // Create a test git repo
        let (_tmp, repo) = setup_test_repo();

        // Create a yak with no tags
        let yak = Yak {
            id: YakId::from("test-yak-a1b2"),
            name: Name::from("test yak"),
            parent_id: None,
            state: YakState::Todo,
            context: None,
            fields: std::collections::HashMap::new(),
            tags: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        // Create a Compacted event with the yak
        let event = crate::domain::YakEvent::Compacted(
            vec![yak],
            vec![],
            crate::domain::event_metadata::EventMetadata::default_legacy(),
        );

        // Build tree from the compacted event
        let tree_oid = build_tree_from_event(&repo, &event, None).unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        // Get the yak subtree
        let yak_subtree = get_yak_subtree(&repo, Some(&tree), "test-yak-a1b2")
            .unwrap()
            .unwrap();

        // Verify .tags blob was NOT created
        assert!(yak_subtree.get_name(".tags").is_none());

        // Read snapshots back from the tree
        let snapshots = read_snapshots_from_tree(&repo, &tree).unwrap();

        // Verify empty tags are preserved as empty vec
        assert_eq!(snapshots.len(), 1);
        let restored_yak = &snapshots[0];
        assert_eq!(restored_yak.tags.len(), 0);
    }

    #[test]
    fn tags_with_whitespace_are_trimmed() {
        // Create a test git repo
        let (_tmp, repo) = setup_test_repo();

        // Create a yak snapshot with tags
        let yak = Yak {
            id: YakId::from("test-yak-a1b2"),
            name: Name::from("test yak"),
            parent_id: None,
            state: YakState::Todo,
            context: None,
            fields: std::collections::HashMap::new(),
            tags: vec![
                "  leading-space".to_string(),
                "trailing-space  ".to_string(),
            ],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        // Create a Compacted event with the yak
        let event = crate::domain::YakEvent::Compacted(
            vec![yak],
            vec![],
            crate::domain::event_metadata::EventMetadata::default_legacy(),
        );

        // Build tree from the compacted event
        let tree_oid = build_tree_from_event(&repo, &event, None).unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        // Read snapshots back from the tree
        let snapshots = read_snapshots_from_tree(&repo, &tree).unwrap();

        // Verify tags are trimmed
        assert_eq!(snapshots.len(), 1);
        let restored_yak = &snapshots[0];
        assert_eq!(restored_yak.tags.len(), 2);
        assert_eq!(restored_yak.tags[0], "leading-space");
        assert_eq!(restored_yak.tags[1], "trailing-space");
    }

    #[test]
    fn removed_yak_ids_are_persisted_in_tree() {
        // Create a test git repo
        let (_tmp, repo) = setup_test_repo();

        // Create a yak snapshot
        let yak = Yak {
            id: YakId::from("kept-yak-a1b2"),
            name: Name::from("kept yak"),
            parent_id: None,
            state: YakState::Todo,
            context: None,
            fields: std::collections::HashMap::new(),
            tags: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        // Create removed yak IDs
        let removed_yak_ids = vec![
            YakId::from("removed-yak-c3d4"),
            YakId::from("removed-yak-e5f6"),
        ];

        // Create a Compacted event with snapshots and removed yak IDs
        let event = crate::domain::YakEvent::Compacted(
            vec![yak],
            removed_yak_ids.clone(),
            crate::domain::event_metadata::EventMetadata::default_legacy(),
        );

        // Build tree from the compacted event
        let tree_oid = build_tree_from_event(&repo, &event, None).unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        // Verify the .removed-yaks blob was written
        let removed_yaks_entry = tree.get_name(".removed-yaks");
        assert!(
            removed_yaks_entry.is_some(),
            ".removed-yaks blob should be written when there are removed yak IDs"
        );

        // Read the blob content
        let blob = repo.find_blob(removed_yaks_entry.unwrap().id()).unwrap();
        let content = std::str::from_utf8(blob.content()).unwrap();

        // Verify it contains the removed yak IDs
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "removed-yak-c3d4");
        assert_eq!(lines[1], "removed-yak-e5f6");
    }

    #[test]
    fn removed_yaks_blob_not_written_when_empty() {
        // Create a test git repo
        let (_tmp, repo) = setup_test_repo();

        // Create a yak snapshot
        let yak = Yak {
            id: YakId::from("test-yak-a1b2"),
            name: Name::from("test yak"),
            parent_id: None,
            state: YakState::Todo,
            context: None,
            fields: std::collections::HashMap::new(),
            tags: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        // Create a Compacted event with no removed yak IDs
        let event = crate::domain::YakEvent::Compacted(
            vec![yak],
            vec![],
            crate::domain::event_metadata::EventMetadata::default_legacy(),
        );

        // Build tree from the compacted event
        let tree_oid = build_tree_from_event(&repo, &event, None).unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        // Verify the .removed-yaks blob was NOT written
        let removed_yaks_entry = tree.get_name(".removed-yaks");
        assert!(
            removed_yaks_entry.is_none(),
            ".removed-yaks blob should not be written when there are no removed yak IDs"
        );
    }
}
