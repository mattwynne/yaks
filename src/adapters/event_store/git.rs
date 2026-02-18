use anyhow::Result;
use git2::Repository;
use std::path::Path;

use crate::domain::ports::{EventStore, EventStoreReader};
use crate::domain::{Yak, YakEvent};

pub struct GitEventStore {
    repo: Repository,
}

impl GitEventStore {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo = Repository::open(repo_path)
            .map_err(|_| anyhow::anyhow!("Error: not in a git repository"))?;
        Ok(Self { repo })
    }

    /// For tests: create from an already-opened Repository
    #[cfg(test)]
    pub fn from_repo(repo: Repository) -> Self {
        Self { repo }
    }

    /// Get the latest commit on refs/notes/yaks, if any
    fn get_latest_commit(&self) -> Result<Option<git2::Commit<'_>>> {
        match self.repo.refname_to_id("refs/notes/yaks") {
            Ok(oid) => Ok(Some(self.repo.find_commit(oid)?)),
            Err(_) => Ok(None),
        }
    }

    /// Get the current tree from refs/notes/yaks, if any
    fn get_current_tree(&self) -> Result<Option<git2::Tree<'_>>> {
        match self.get_latest_commit()? {
            Some(commit) => Ok(Some(commit.tree()?)),
            None => Ok(None),
        }
    }

    /// Create a tree for a single yak with initial files
    fn create_yak_tree(&self, name: &str, state: &str, context: &str) -> Result<git2::Oid> {
        let mut builder = self.repo.treebuilder(None)?;

        let state_blob = self.repo.blob(state.as_bytes())?;
        builder.insert("state", state_blob, 0o100644)?;

        let context_blob = self.repo.blob(context.as_bytes())?;
        builder.insert("context.md", context_blob, 0o100644)?;

        let name_blob = self.repo.blob(name.as_bytes())?;
        builder.insert("name", name_blob, 0o100644)?;

        Ok(builder.write()?)
    }

    /// Get a yak's subtree from the root tree
    fn get_yak_subtree(
        &self,
        root: Option<&git2::Tree>,
        yak_name: &str,
    ) -> Result<Option<git2::Tree<'_>>> {
        let Some(root) = root else {
            return Ok(None);
        };

        let parts: Vec<&str> = yak_name.split('/').collect();
        let mut current_oid = root.id();

        for part in &parts {
            let tree = self.repo.find_tree(current_oid)?;
            let entry_oid = match tree.get_name(part) {
                Some(entry) => entry.id(),
                None => return Ok(None),
            };

            // Verify it's a tree
            let obj = self.repo.find_object(entry_oid, None)?;
            if obj.kind() != Some(git2::ObjectType::Tree) {
                anyhow::bail!("Expected tree entry for '{}'", part);
            }

            current_oid = entry_oid;
        }

        Ok(Some(self.repo.find_tree(current_oid)?))
    }

    /// Recursively search the git tree for a directory entry matching
    /// the given yak ID. Returns the full path (e.g., "parent-a1b2/child-c3d4").
    /// This is needed because events only contain the yak's own ID, but
    /// the git tree nests children under their parent's directory.
    fn find_yak_path(&self, root: Option<&git2::Tree>, id: &str) -> Option<String> {
        let root = root?;
        self.find_yak_path_recursive(root, id, "")
    }

    fn find_yak_path_recursive(&self, tree: &git2::Tree, id: &str, prefix: &str) -> Option<String> {
        for entry in tree.iter() {
            if entry.kind() != Some(git2::ObjectType::Tree) {
                continue;
            }
            let name = entry.name()?;
            let full_path = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };

            if name == id {
                // Verify this is a yak directory (has a state or context.md blob)
                if let Ok(subtree) = self.repo.find_tree(entry.id()) {
                    if subtree.get_name("state").is_some()
                        || subtree.get_name("context.md").is_some()
                    {
                        return Some(full_path);
                    }
                }
            }

            // Recurse into subtrees to find nested yaks
            if let Ok(subtree) = self.repo.find_tree(entry.id()) {
                if let Some(found) = self.find_yak_path_recursive(&subtree, id, &full_path) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Resolve a yak ID to its full tree path.
    /// If the ID already contains a slash (explicit path), use it directly.
    /// Otherwise, search the tree recursively.
    /// Falls back to the bare ID if not found (for new entries).
    fn resolve_yak_path(&self, tree: Option<&git2::Tree>, id: &str) -> String {
        if id.contains('/') {
            return id.to_string();
        }
        self.find_yak_path(tree, id)
            .unwrap_or_else(|| id.to_string())
    }

    /// Update a file in a yak's subtree, returning new root tree OID
    fn update_yak_file(
        &self,
        current_tree: Option<&git2::Tree>,
        yak_name: &str,
        file_name: &str,
        content: &str,
    ) -> Result<git2::Oid> {
        let blob_oid = self.repo.blob(content.as_bytes())?;

        // Build the yak's subtree
        let yak_subtree = self.get_yak_subtree(current_tree, yak_name)?;
        let mut yak_builder = self.repo.treebuilder(yak_subtree.as_ref())?;
        yak_builder.insert(file_name, blob_oid, 0o100644)?;
        let yak_tree_oid = yak_builder.write()?;

        // Rebuild root tree with updated yak subtree
        self.set_yak_in_root(current_tree, yak_name, Some(yak_tree_oid))
    }

    /// Set (or remove) a yak subtree in the root tree, handling
    /// hierarchical names by rebuilding intermediate trees.
    fn set_yak_in_root(
        &self,
        root: Option<&git2::Tree>,
        yak_name: &str,
        subtree_oid: Option<git2::Oid>,
    ) -> Result<git2::Oid> {
        let parts: Vec<&str> = yak_name.split('/').collect();

        if parts.len() == 1 {
            // Simple case: direct child of root
            let mut builder = self.repo.treebuilder(root)?;
            match subtree_oid {
                Some(oid) => {
                    builder.insert(parts[0], oid, 0o040000)?;
                }
                None => {
                    let _ = builder.remove(parts[0]);
                }
            }
            return Ok(builder.write()?);
        }

        // Hierarchical case: need to rebuild intermediate trees
        let intermediate_name = parts[0];
        let rest = parts[1..].join("/");

        let intermediate_tree = root
            .and_then(|r| r.get_name(intermediate_name))
            .map(|entry| self.repo.find_tree(entry.id()))
            .transpose()?;

        let new_intermediate =
            self.set_yak_in_root(intermediate_tree.as_ref(), &rest, subtree_oid)?;

        let mut root_builder = self.repo.treebuilder(root)?;
        root_builder.insert(intermediate_name, new_intermediate, 0o040000)?;
        Ok(root_builder.write()?)
    }

    /// Build an updated tree by applying an event to the current tree.
    /// All operations happen in git's object database - no filesystem IO.
    fn build_tree_from_event(
        &self,
        event: &YakEvent,
        current_tree: Option<&git2::Tree>,
    ) -> Result<git2::Oid> {
        match event {
            YakEvent::Added(e, metadata) => {
                let yak_tree_oid = self.create_yak_tree(e.name.as_str(), "todo", "")?;
                // Add .metadata.json to the yak subtree
                let metadata_json = serde_json::json!({
                    "created_by": {
                        "name": metadata.author.name,
                        "email": metadata.author.email
                    },
                    "created_at": metadata.timestamp.as_epoch_secs()
                });
                let metadata_blob = self.repo.blob(metadata_json.to_string().as_bytes())?;
                let subtree = self.repo.find_tree(yak_tree_oid)?;
                let mut builder = self.repo.treebuilder(Some(&subtree))?;
                builder.insert(".metadata.json", metadata_blob, 0o100644)?;
                let updated_tree_oid = builder.write()?;
                let path = match &e.parent_id {
                    Some(parent) => format!("{}/{}", parent, e.id),
                    None => e.id.to_string(),
                };
                self.set_yak_in_root(current_tree, &path, Some(updated_tree_oid))
            }

            YakEvent::Removed(e, _) => {
                let path = self.resolve_yak_path(current_tree, e.id.as_str());
                self.set_yak_in_root(current_tree, &path, None)
            }

            YakEvent::Moved(e, _) => {
                // Move yak subtree to new parent
                let old_path = self.resolve_yak_path(current_tree, e.id.as_str());
                let old_subtree_oid = self
                    .get_yak_subtree(current_tree, &old_path)?
                    .map(|t| t.id());

                let intermediate = self.set_yak_in_root(current_tree, &old_path, None)?;
                let intermediate_tree = self.repo.find_tree(intermediate)?;

                // Place under new parent if specified
                let target = match &e.new_parent {
                    Some(parent) => format!("{}/{}", parent, e.id),
                    None => e.id.to_string(),
                };
                self.set_yak_in_root(Some(&intermediate_tree), &target, old_subtree_oid)
            }

            YakEvent::FieldUpdated(e, _) => {
                let path = self.resolve_yak_path(current_tree, e.id.as_str());
                self.update_yak_file(current_tree, &path, &e.field_name, &e.content)
            }
        }
    }
    /// Read the current git tree state and synthesize domain events.
    /// All yak IDs are regenerated using `generate_id(name, parent_id)`,
    /// making this suitable for repairing inconsistent data.
    pub fn snapshot_events(&self) -> Result<Vec<YakEvent>> {
        let tree = self.get_current_tree()?;
        let Some(tree) = tree else {
            return Ok(Vec::new());
        };

        let mut events = Vec::new();
        self.collect_snapshot_events(&tree, None, &mut events)?;
        Ok(events)
    }

    fn collect_snapshot_events(
        &self,
        tree: &git2::Tree,
        parent_id: Option<&crate::domain::slug::YakId>,
        events: &mut Vec<YakEvent>,
    ) -> Result<()> {
        use crate::domain::field::RESERVED_FIELDS;
        use crate::domain::slug::{generate_id, Name};

        for entry in tree.iter() {
            if entry.kind() != Some(git2::ObjectType::Tree) {
                continue;
            }
            let entry_name = match entry.name() {
                Some(n) => n.to_string(),
                None => continue,
            };

            let subtree = self.repo.find_tree(entry.id())?;

            // A yak subtree has a `state` or `context.md` blob
            let is_yak =
                subtree.get_name("state").is_some() || subtree.get_name("context.md").is_some();
            if !is_yak {
                continue;
            }

            // Read name from `name` blob, falling back to directory entry name
            let name_str = if let Some(name_entry) = subtree.get_name("name") {
                let name_blob = self.repo.find_blob(name_entry.id())?;
                std::str::from_utf8(name_blob.content())?.trim().to_string()
            } else {
                entry_name.clone()
            };
            let name = Name::from(name_str.as_str());
            let id = generate_id(&name_str, parent_id);

            // Read .metadata.json if present
            let added_metadata =
                if let Some(meta_entry) = subtree.get_name(".metadata.json") {
                    if let Ok(meta_blob) = self.repo.find_blob(meta_entry.id()) {
                        if let Ok(content) = std::str::from_utf8(meta_blob.content()) {
                            if let Ok(json) =
                                serde_json::from_str::<serde_json::Value>(content)
                            {
                                use crate::domain::event_metadata::{
                                    Author, EventMetadata, Timestamp,
                                };
                                EventMetadata::new(
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
                                    Timestamp(
                                        json["created_at"].as_i64().unwrap_or(0),
                                    ),
                                )
                            } else {
                                crate::domain::event_metadata::EventMetadata::default_legacy()
                            }
                        } else {
                            crate::domain::event_metadata::EventMetadata::default_legacy()
                        }
                    } else {
                        crate::domain::event_metadata::EventMetadata::default_legacy()
                    }
                } else {
                    crate::domain::event_metadata::EventMetadata::default_legacy()
                };

            // Added event
            events.push(YakEvent::Added(
                crate::domain::events::AddedEvent {
                    name: name.clone(),
                    id: id.clone(),
                    parent_id: parent_id.cloned(),
                },
                added_metadata,
            ));

            // State
            if let Some(state_entry) = subtree.get_name("state") {
                let state_blob = self.repo.find_blob(state_entry.id())?;
                let state = std::str::from_utf8(state_blob.content())?.trim();
                if state != "todo" {
                    events.push(YakEvent::FieldUpdated(
                        crate::domain::events::FieldUpdatedEvent {
                            id: id.clone(),
                            field_name: "state".to_string(),
                            content: state.to_string(),
                        },
                        crate::domain::event_metadata::EventMetadata::default_legacy(),
                    ));
                }
            }

            // Context
            if let Some(context_entry) = subtree.get_name("context.md") {
                let context_blob = self.repo.find_blob(context_entry.id())?;
                let content = std::str::from_utf8(context_blob.content())?;
                if !content.is_empty() {
                    events.push(YakEvent::FieldUpdated(
                        crate::domain::events::FieldUpdatedEvent {
                            id: id.clone(),
                            field_name: "context.md".to_string(),
                            content: content.to_string(),
                        },
                        crate::domain::event_metadata::EventMetadata::default_legacy(),
                    ));
                }
            }

            // Custom fields
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
                let field_blob = self.repo.find_blob(field_entry.id())?;
                let content = std::str::from_utf8(field_blob.content())?;
                events.push(YakEvent::FieldUpdated(
                    crate::domain::events::FieldUpdatedEvent {
                        id: id.clone(),
                        field_name: field_name.to_string(),
                        content: content.to_string(),
                    },
                    crate::domain::event_metadata::EventMetadata::default_legacy(),
                ));
            }

            // Recurse into children (subtrees within this yak's subtree)
            self.collect_snapshot_events(&subtree, Some(&id), events)?;
        }

        Ok(())
    }
}

#[cfg(test)]
impl GitEventStore {
    pub fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
        Ok(EventStore::get_all_events(self)?
            .into_iter()
            .filter(|e| e.yak_id() == name)
            .collect())
    }
}

impl EventStore for GitEventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()> {
        let current_tree = self.get_current_tree()?;

        let tree_oid = self.build_tree_from_event(event, current_tree.as_ref())?;
        let tree = self.repo.find_tree(tree_oid)?;

        let message = event.format_message();

        let parent = self.get_latest_commit()?;
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        let meta = event.metadata();
        let author_name = if meta.author.name.is_empty() {
            "yx"
        } else {
            &meta.author.name
        };
        let author_email = if meta.author.email.is_empty() {
            "yx@localhost"
        } else {
            &meta.author.email
        };
        let time = git2::Time::new(meta.timestamp.as_epoch_secs(), 0);
        let sig = git2::Signature::new(author_name, author_email, &time)?;

        self.repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            &message,
            &tree,
            &parents,
        )?;

        Ok(())
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        let Some(latest) = self.get_latest_commit()? else {
            return Ok(Vec::new());
        };

        let mut events = Vec::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
        revwalk.push(latest.id())?;

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let message = commit.message().unwrap_or("").trim();

            if message.is_empty() {
                continue;
            }

            match YakEvent::parse(message) {
                Ok(event) => {
                    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
                    let author = Author {
                        name: commit.author().name().unwrap_or("unknown").to_string(),
                        email: commit.author().email().unwrap_or("").to_string(),
                    };
                    let timestamp = Timestamp(commit.author().when().seconds());
                    let metadata = EventMetadata::new(author, timestamp);
                    events.push(event.with_metadata(metadata));
                }
                Err(_) => continue, // Skip unparseable commits
            }
        }

        // Reverse: revwalk gives newest-first, we want chronological
        events.reverse();
        Ok(events)
    }

    fn reset_from_snapshot(&mut self, yaks: &[Yak]) -> Result<usize> {
        use super::migration::CURRENT_SCHEMA_VERSION;
        use crate::domain::slug::YakId;
        use std::collections::HashMap;

        // Build a YakId→Yak index
        let yak_map: HashMap<&YakId, &Yak> = yaks.iter().map(|y| (&y.id, y)).collect();

        // Find root yaks (those whose ID doesn't appear in any children list)
        let mut child_ids = std::collections::HashSet::new();
        for yak in yaks {
            for child_id in &yak.children {
                child_ids.insert(child_id);
            }
        }
        let roots: Vec<&Yak> = yaks.iter().filter(|y| !child_ids.contains(&y.id)).collect();

        // Recursively build tree
        fn build_yak_subtree(
            repo: &Repository,
            yak: &Yak,
            yak_map: &HashMap<&YakId, &Yak>,
        ) -> Result<git2::Oid> {
            let mut builder = repo.treebuilder(None)?;

            // Add standard blobs
            let state_blob = repo.blob(yak.state.as_bytes())?;
            builder.insert("state", state_blob, 0o100644)?;

            let context_content = yak.context.as_deref().unwrap_or("");
            let context_blob = repo.blob(context_content.as_bytes())?;
            builder.insert("context.md", context_blob, 0o100644)?;

            let name_blob = repo.blob(yak.name.as_str().as_bytes())?;
            builder.insert("name", name_blob, 0o100644)?;

            let id_blob = repo.blob(yak.id.as_str().as_bytes())?;
            builder.insert("id", id_blob, 0o100644)?;

            // Add custom fields
            for (field_name, content) in &yak.fields {
                let field_blob = repo.blob(content.as_bytes())?;
                builder.insert(field_name, field_blob, 0o100644)?;
            }

            // Add children subtrees
            for child_id in &yak.children {
                let child = yak_map.get(child_id).ok_or_else(|| {
                    anyhow::anyhow!(
                        "child yak '{}' referenced by '{}' not found in snapshot",
                        child_id,
                        yak.id
                    )
                })?;
                let child_tree = build_yak_subtree(repo, child, yak_map)?;
                builder.insert(child_id.as_str(), child_tree, 0o040000)?;
            }

            Ok(builder.write()?)
        }

        // Build root tree
        let mut root_builder = self.repo.treebuilder(None)?;

        for root in roots {
            let yak_tree = build_yak_subtree(&self.repo, root, &yak_map)?;
            root_builder.insert(root.id.as_str(), yak_tree, 0o040000)?;
        }

        // Add .schema-version
        let version_blob = self
            .repo
            .blob(CURRENT_SCHEMA_VERSION.to_string().as_bytes())?;
        root_builder.insert(".schema-version", version_blob, 0o100644)?;

        let tree_oid = root_builder.write()?;
        let tree = self.repo.find_tree(tree_oid)?;

        // Create commit
        let parent = self.get_latest_commit()?;
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        let sig = self
            .repo
            .signature()
            .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;

        self.repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Snapshot: rebuilt from disk",
            &tree,
            &parents,
        )?;

        Ok(yaks.len())
    }
}

impl EventStoreReader for GitEventStore {
    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        EventStore::get_all_events(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
    use crate::domain::events::FieldUpdatedEvent;
    use crate::domain::slug::{Name, YakId};
    use crate::domain::AddedEvent;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitEventStore) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        // Configure git user for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        let store = GitEventStore::from_repo(repo);
        (tmp, store)
    }

    #[test]
    fn append_creates_commit_on_refs_notes_yaks() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Verify ref exists
        let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = store.repo.find_commit(oid).unwrap();
        assert_eq!(commit.message().unwrap(), "Added: \"test\" \"test-a1b2\"");
    }

    #[test]
    fn added_with_id_keys_tree_entry_by_id() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Tree entry should be keyed by id, not name
        assert!(
            tree.get_name("test-a1b2").is_some(),
            "Expected tree entry keyed by id 'test-a1b2'"
        );
        assert!(
            tree.get_name("test").is_none(),
            "Should not have tree entry keyed by name 'test'"
        );
    }

    #[test]
    fn state_update_after_add_uses_same_tree_entry() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: "state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Should have exactly one entry, keyed by id
        assert_eq!(
            tree.len(),
            1,
            "Expected exactly 1 tree entry, got {}",
            tree.len()
        );

        let entry = tree.get_name("test-a1b2").unwrap();
        let subtree = entry.to_object(&store.repo).unwrap();
        let subtree = subtree.as_tree().unwrap();

        // Verify state was updated
        let state_entry = subtree.get_name("state").unwrap();
        let state_blob = state_entry.to_object(&store.repo).unwrap();
        let state_content = std::str::from_utf8(state_blob.as_blob().unwrap().content()).unwrap();
        assert_eq!(state_content, "wip");
    }

    #[test]
    fn added_with_parent_id_nests_under_parent() {
        let (_tmp, mut store) = setup_test_repo();

        // Add parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Add child under parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Root should have one entry: the parent
        assert_eq!(tree.len(), 1);

        let parent_entry = tree.get_name("parent-a1b2").unwrap();
        let parent_tree = parent_entry.to_object(&store.repo).unwrap();
        let parent_tree = parent_tree.as_tree().unwrap();

        // Parent tree should have its own files + child subtree
        assert!(
            parent_tree.get_name("child-c3d4").is_some(),
            "Expected child subtree under parent"
        );
        assert!(
            parent_tree.get_name("state").is_some(),
            "Expected parent's state file"
        );
    }

    #[test]
    fn snapshot_events_synthesizes_added_for_each_yak() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        // Should have an Added event with regenerated ID
        let added = events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(_, _)))
            .unwrap();
        if let YakEvent::Added(e, _) = added {
            assert_eq!(e.name, Name::from("test"));
            assert!(
                e.id.as_str().starts_with("test-"),
                "Expected regenerated ID starting with 'test-', got '{}'",
                e.id
            );
            assert!(e.parent_id.is_none());
        }
    }

    #[test]
    fn snapshot_events_includes_state_and_context() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: "state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: "context.md".to_string(),
                    content: "some notes".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        assert!(
            events
                .iter()
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == "state" && f.content == "wip")),
            "Expected FieldUpdated event for state 'wip'"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == "context.md" && f.content == "some notes")),
            "Expected FieldUpdated event for context.md"
        );
    }

    #[test]
    fn snapshot_events_skips_state_when_todo() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == "state")),
            "Should not emit FieldUpdated for state when state is 'todo'"
        );
    }

    #[test]
    fn snapshot_events_regenerates_ids_for_legacy_yaks() {
        let (_tmp, mut store) = setup_test_repo();

        // Simulate a legacy yak where the tree key is a plain slug
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("dx"),
                    id: YakId::from("dx"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();
        let added = events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(_, _)))
            .unwrap();

        if let YakEvent::Added(e, _) = added {
            // Should get a proper ID with suffix, not plain "dx"
            assert!(
                e.id.as_str().starts_with("dx-") && e.id.as_str().len() > 3,
                "Expected regenerated ID like 'dx-xxxx', got '{}'",
                e.id
            );
        }
    }

    #[test]
    fn snapshot_events_handles_nested_yaks() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();
        let added_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, YakEvent::Added(_, _)))
            .collect();

        assert_eq!(added_events.len(), 2, "Expected 2 Added events");

        // Child should have a parent_id matching the regenerated parent ID
        if let (YakEvent::Added(parent, _), YakEvent::Added(child, _)) =
            (&added_events[0], &added_events[1])
        {
            assert!(parent.parent_id.is_none());
            assert_eq!(child.parent_id.as_ref(), Some(&parent.id));
        }
    }

    #[test]
    fn snapshot_events_includes_custom_fields() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                crate::domain::events::FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: "plan".to_string(),
                    content: "step 1".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        assert!(
            events.iter().any(
                |e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == "plan" && f.content == "step 1")
            ),
            "Expected FieldUpdated event for 'plan'"
        );
    }

    #[test]
    fn snapshot_events_empty_tree() {
        let (_tmp, store) = setup_test_repo();
        let events = store.snapshot_events().unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn snapshot_events_reads_metadata_from_tree() {
        use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

        let (_tmp, mut store) = setup_test_repo();

        let metadata = EventMetadata::new(
            Author {
                name: "Snapshot Author".to_string(),
                email: "snap@test.com".to_string(),
            },
            Timestamp(1708300800),
        );

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                metadata.clone(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();
        let added = events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(..)))
            .unwrap();
        assert_eq!(added.metadata().author.name, "Snapshot Author");
        assert_eq!(added.metadata().timestamp, Timestamp(1708300800));
    }

    #[test]
    fn reset_from_snapshot_builds_correct_tree() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let mut fields = HashMap::new();
        fields.insert("plan".to_string(), "step 1".to_string());

        let yak1 = Yak {
            id: YakId::from("yak1-a1b2"),
            name: Name::from("First Yak"),
            parent_id: None,
            state: "todo".to_string(),
            context: Some("some context".to_string()),
            fields: fields.clone(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let yak2 = Yak {
            id: YakId::from("yak2-c3d4"),
            name: Name::from("Second Yak"),
            parent_id: None,
            state: "wip".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let count = store.reset_from_snapshot(&[yak1, yak2]).unwrap();
        assert_eq!(count, 2);

        // Verify the git tree structure
        let tree = store.get_current_tree().unwrap().unwrap();

        // Check yak1 subtree
        let yak1_entry = tree.get_name("yak1-a1b2").unwrap();
        let yak1_tree = store.repo.find_tree(yak1_entry.id()).unwrap();

        let state_blob = yak1_tree.get_name("state").unwrap();
        let state = store.repo.find_blob(state_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(state.content()).unwrap(), "todo");

        let context_blob = yak1_tree.get_name("context.md").unwrap();
        let context = store.repo.find_blob(context_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(context.content()).unwrap(),
            "some context"
        );

        let name_blob = yak1_tree.get_name("name").unwrap();
        let name = store.repo.find_blob(name_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(name.content()).unwrap(), "First Yak");

        let id_blob = yak1_tree.get_name("id").unwrap();
        let id = store.repo.find_blob(id_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(id.content()).unwrap(), "yak1-a1b2");

        let plan_blob = yak1_tree.get_name("plan").unwrap();
        let plan = store.repo.find_blob(plan_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(plan.content()).unwrap(), "step 1");

        // Check yak2 subtree
        let yak2_entry = tree.get_name("yak2-c3d4").unwrap();
        let yak2_tree = store.repo.find_tree(yak2_entry.id()).unwrap();

        let state2_blob = yak2_tree.get_name("state").unwrap();
        let state2 = store.repo.find_blob(state2_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(state2.content()).unwrap(), "wip");

        let context2_blob = yak2_tree.get_name("context.md").unwrap();
        let context2 = store.repo.find_blob(context2_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(context2.content()).unwrap(), "");

        // Check .schema-version
        let schema_blob = tree.get_name(".schema-version").unwrap();
        let schema = store.repo.find_blob(schema_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(schema.content()).unwrap(), "3");
    }

    #[test]
    fn reset_from_snapshot_handles_children() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let child = Yak {
            id: YakId::from("child-x1y2"),
            name: Name::from("Child Yak"),
            parent_id: None,
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let parent = Yak {
            id: YakId::from("parent-a1b2"),
            name: Name::from("Parent Yak"),
            parent_id: None,
            state: "wip".to_string(),
            context: Some("parent context".to_string()),
            fields: HashMap::new(),
            children: vec![YakId::from("child-x1y2")],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        store.reset_from_snapshot(&[parent, child]).unwrap();

        // Verify tree structure
        let tree = store.get_current_tree().unwrap().unwrap();

        // Root should only have parent (child is nested)
        assert_eq!(tree.len(), 2); // parent + .schema-version

        // Get parent subtree
        let parent_entry = tree.get_name("parent-a1b2").unwrap();
        let parent_tree = store.repo.find_tree(parent_entry.id()).unwrap();

        // Parent should have its own blobs + child subtree
        assert!(parent_tree.get_name("state").is_some());
        assert!(parent_tree.get_name("context.md").is_some());
        assert!(parent_tree.get_name("name").is_some());
        assert!(parent_tree.get_name("id").is_some());

        // Child should be nested under parent
        let child_entry = parent_tree.get_name("child-x1y2").unwrap();
        let child_tree = store.repo.find_tree(child_entry.id()).unwrap();

        // Verify child blobs
        let child_name_blob = child_tree.get_name("name").unwrap();
        let child_name = store.repo.find_blob(child_name_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(child_name.content()).unwrap(),
            "Child Yak"
        );
    }

    #[test]
    fn reset_from_snapshot_parents_to_existing_commit() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        // First, add a yak via append (creates a commit)
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("initial"),
                    id: YakId::from("initial-z9z9"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let first_commit_oid = store.get_latest_commit().unwrap().unwrap().id();

        // Now call reset_from_snapshot
        let yak = Yak {
            id: YakId::from("snapshot-a1b2"),
            name: Name::from("Snapshot Yak"),
            parent_id: None,
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        store.reset_from_snapshot(&[yak]).unwrap();

        // Verify the new commit has a parent
        let latest = store.get_latest_commit().unwrap().unwrap();
        assert_eq!(latest.parent_count(), 1);
        assert_eq!(latest.parent_id(0).unwrap(), first_commit_oid);
    }

    #[test]
    fn reset_from_snapshot_returns_yak_count() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let yak1 = Yak {
            id: YakId::from("yak1-a1b2"),
            name: Name::from("Yak One"),
            parent_id: None,
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let yak2 = Yak {
            id: YakId::from("yak2-c3d4"),
            name: Name::from("Yak Two"),
            parent_id: None,
            state: "wip".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let yak3 = Yak {
            id: YakId::from("yak3-e5f6"),
            name: Name::from("Yak Three"),
            parent_id: None,
            state: "done".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let count = store.reset_from_snapshot(&[yak1, yak2, yak3]).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn reset_from_snapshot_empty_list() {
        let (_tmp, mut store) = setup_test_repo();

        let count = store.reset_from_snapshot(&[]).unwrap();
        assert_eq!(count, 0);

        // Verify a commit was created
        let tree = store.get_current_tree().unwrap().unwrap();

        // Should only have .schema-version
        assert_eq!(tree.len(), 1);
        let schema_blob = tree.get_name(".schema-version").unwrap();
        let schema = store.repo.find_blob(schema_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(schema.content()).unwrap(), "3");
    }

    #[test]
    fn rename_nested_yak_updates_correct_entry() {
        let (_tmp, mut store) = setup_test_repo();

        // Add parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Add child under parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Rename the child
        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("child-c3d4"),
                    field_name: "name".to_string(),
                    content: "renamed child".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Root should still have one entry: the parent (no orphan at root)
        let root_entries: Vec<_> = tree
            .iter()
            .filter(|e| e.kind() == Some(git2::ObjectType::Tree))
            .collect();
        assert_eq!(
            root_entries.len(),
            1,
            "Expected 1 root tree entry, got {} (orphan created?)",
            root_entries.len()
        );

        // Verify the child's name was updated in the nested position
        let parent_entry = tree.get_name("parent-a1b2").unwrap();
        let parent_tree = store.repo.find_tree(parent_entry.id()).unwrap();
        let child_entry = parent_tree.get_name("child-c3d4").unwrap();
        let child_tree = store.repo.find_tree(child_entry.id()).unwrap();

        let name_blob = child_tree.get_name("name").unwrap();
        let name = store.repo.find_blob(name_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(name.content()).unwrap(),
            "renamed child"
        );
    }

    #[test]
    fn state_update_nested_yak_updates_correct_entry() {
        let (_tmp, mut store) = setup_test_repo();

        // Add parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Add child under parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Update state of nested child
        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("child-c3d4"),
                    field_name: "state".to_string(),
                    content: "done".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Root should still have one entry: the parent (no orphan)
        let root_entries: Vec<_> = tree
            .iter()
            .filter(|e| e.kind() == Some(git2::ObjectType::Tree))
            .collect();
        assert_eq!(root_entries.len(), 1);

        // Verify state was updated in the nested position
        let parent_entry = tree.get_name("parent-a1b2").unwrap();
        let parent_tree = store.repo.find_tree(parent_entry.id()).unwrap();
        let child_entry = parent_tree.get_name("child-c3d4").unwrap();
        let child_tree = store.repo.find_tree(child_entry.id()).unwrap();

        let state_blob = child_tree.get_name("state").unwrap();
        let state = store.repo.find_blob(state_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(state.content()).unwrap(), "done");
    }

    #[test]
    fn append_uses_event_metadata_for_commit_signature() {
        use crate::domain::event_metadata::{Author, Timestamp};

        let (_tmp, mut store) = setup_test_repo();

        let metadata = EventMetadata::new(
            Author {
                name: "Custom Author".to_string(),
                email: "custom@example.com".to_string(),
            },
            Timestamp(1708300800),
        );

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                metadata,
            ))
            .unwrap();

        let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = store.repo.find_commit(oid).unwrap();
        assert_eq!(commit.author().name().unwrap(), "Custom Author");
        assert_eq!(commit.author().email().unwrap(), "custom@example.com");
        assert_eq!(commit.author().when().seconds(), 1708300800);
    }

    #[test]
    fn get_all_events_populates_metadata_from_commits() {
        use crate::domain::event_metadata::{Author, Timestamp};

        let (_tmp, mut store) = setup_test_repo();

        let metadata = EventMetadata::new(
            Author {
                name: "Reader Test".to_string(),
                email: "reader@test.com".to_string(),
            },
            Timestamp(1708300800),
        );

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                metadata.clone(),
            ))
            .unwrap();

        let events = EventStore::get_all_events(&store).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].metadata().author.name, "Reader Test");
        assert_eq!(events[0].metadata().author.email, "reader@test.com");
        assert_eq!(events[0].metadata().timestamp, Timestamp(1708300800));
    }

    #[test]
    fn reset_from_snapshot_errors_on_missing_child() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let parent = Yak {
            id: YakId::from("parent-a1b2"),
            name: Name::from("Parent Yak"),
            parent_id: None,
            state: "wip".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![YakId::from("missing-child-x1y2")], // child doesn't exist
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let result = store.reset_from_snapshot(&[parent]);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("missing-child-x1y2"));
        assert!(err_msg.contains("parent-a1b2"));
    }
}
