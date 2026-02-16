use anyhow::Result;
use git2::Repository;
use std::path::Path;

use crate::domain::ports::{EventStore, EventStoreReader};
use crate::domain::YakEvent;

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
            YakEvent::Added(e) => {
                let yak_tree_oid = self.create_yak_tree(&e.name, "todo", "")?;
                let key = if e.id.is_empty() { &e.name } else { &e.id };
                let path = match &e.parent_id {
                    Some(parent) => format!("{}/{}", parent, key),
                    None => key.to_string(),
                };
                self.set_yak_in_root(current_tree, &path, Some(yak_tree_oid))
            }

            YakEvent::Removed(e) => self.set_yak_in_root(current_tree, &e.id, None),

            YakEvent::Moved(e) => {
                // Move yak subtree to new parent
                // For now, just update the tree location
                let old_subtree_oid = self.get_yak_subtree(current_tree, &e.id)?.map(|t| t.id());

                let intermediate = self.set_yak_in_root(current_tree, &e.id, None)?;
                let intermediate_tree = self.repo.find_tree(intermediate)?;

                // Place under new parent if specified
                let target = match &e.new_parent {
                    Some(parent) => format!("{}/{}", parent, e.id),
                    None => e.id.clone(),
                };
                self.set_yak_in_root(Some(&intermediate_tree), &target, old_subtree_oid)
            }

            YakEvent::Renamed(e) => {
                // Update name file for renamed yak
                self.update_yak_file(current_tree, &e.id, "name", &e.new_name)
            }

            YakEvent::ContextUpdated(e) => {
                self.update_yak_file(current_tree, &e.id, "context.md", &e.content)
            }

            YakEvent::StateUpdated(e) => {
                self.update_yak_file(current_tree, &e.id, "state", &e.state)
            }

            YakEvent::FieldUpdated(e) => {
                self.update_yak_file(current_tree, &e.id, &e.field_name, &e.content)
            }
        }
    }
    /// Materialize the git tree at HEAD of refs/notes/yaks to a filesystem path.
    /// Removes existing yak entries, then walks the tree recursively,
    /// writing blobs as files and creating directories for subtrees.
    pub fn materialize_tree(&self, target: &Path) -> Result<()> {
        let tree = self.get_current_tree()?;
        let Some(tree) = tree else {
            anyhow::bail!("No yaks tree found in refs/notes/yaks");
        };

        // Remove existing entries that correspond to tree entries,
        // but preserve non-yak files (like .git, .gitignore)
        if target.exists() {
            for entry in tree.iter() {
                if let Some(name) = entry.name() {
                    let path = target.join(name);
                    if path.exists() {
                        if path.is_dir() {
                            std::fs::remove_dir_all(&path)?;
                        } else {
                            std::fs::remove_file(&path)?;
                        }
                    }
                }
            }
        }
        std::fs::create_dir_all(target)?;

        self.write_tree_to_dir(&tree, target)
    }

    fn write_tree_to_dir(&self, tree: &git2::Tree, dir: &Path) -> Result<()> {
        for entry in tree.iter() {
            let name = entry
                .name()
                .ok_or_else(|| anyhow::anyhow!("Tree entry has no name"))?;
            let path = dir.join(name);

            match entry.kind() {
                Some(git2::ObjectType::Blob) => {
                    let blob = self.repo.find_blob(entry.id())?;
                    std::fs::write(&path, blob.content())?;
                }
                Some(git2::ObjectType::Tree) => {
                    std::fs::create_dir_all(&path)?;
                    let subtree = self.repo.find_tree(entry.id())?;
                    self.write_tree_to_dir(&subtree, &path)?;
                }
                _ => {} // Skip other object types
            }
        }
        Ok(())
    }
}

#[cfg(test)]
impl GitEventStore {
    pub fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
        Ok(EventStore::get_all_events(self)?
            .into_iter()
            .filter(|e| e.yak_name() == name)
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

        let sig = self
            .repo
            .signature()
            .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;

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
                Ok(event) => events.push(event),
                Err(_) => continue, // Skip unparseable commits
            }
        }

        // Reverse: revwalk gives newest-first, we want chronological
        events.reverse();
        Ok(events)
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
    use crate::domain::{AddedEvent, StateUpdatedEvent};
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
            .append(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: "test-a1b2".to_string(),
                parent_id: None,
            }))
            .unwrap();

        // Verify ref exists
        let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = store.repo.find_commit(oid).unwrap();
        assert_eq!(commit.message().unwrap(), "Added: \"test\" \"test-a1b2\"");
    }

    #[test]
    fn append_builds_tree_with_yak_directory() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: String::new(),
                parent_id: None,
            }))
            .unwrap();

        let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = store.repo.find_commit(oid).unwrap();
        let tree = commit.tree().unwrap();

        // Verify test/ directory exists in tree
        let entry = tree.get_name("test").unwrap();
        let subtree = entry.to_object(&store.repo).unwrap();
        let subtree = subtree.as_tree().unwrap();

        // Verify state file
        let state_entry = subtree.get_name("state").unwrap();
        let state_blob = state_entry.to_object(&store.repo).unwrap();
        let state_content = std::str::from_utf8(state_blob.as_blob().unwrap().content()).unwrap();
        assert_eq!(state_content, "todo");
    }

    #[test]
    fn added_with_id_keys_tree_entry_by_id() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: "test-a1b2".to_string(),
                parent_id: None,
            }))
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
            .append(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: "test-a1b2".to_string(),
                parent_id: None,
            }))
            .unwrap();

        store
            .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                id: "test-a1b2".to_string(),
                state: "wip".to_string(),
            }))
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
            .append(&YakEvent::Added(AddedEvent {
                name: "parent".to_string(),
                id: "parent-a1b2".to_string(),
                parent_id: None,
            }))
            .unwrap();

        // Add child under parent
        store
            .append(&YakEvent::Added(AddedEvent {
                name: "child".to_string(),
                id: "child-c3d4".to_string(),
                parent_id: Some("parent-a1b2".to_string()),
            }))
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
    fn materialize_tree_round_trips() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(AddedEvent {
                name: "test".to_string(),
                id: "test-a1b2".to_string(),
                parent_id: None,
            }))
            .unwrap();

        store
            .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                id: "test-a1b2".to_string(),
                state: "wip".to_string(),
            }))
            .unwrap();

        let target = _tmp.path().join("materialized");
        store.materialize_tree(&target).unwrap();

        // Verify directory structure
        assert!(target.join("test-a1b2").is_dir());
        assert_eq!(
            std::fs::read_to_string(target.join("test-a1b2/state")).unwrap(),
            "wip"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("test-a1b2/name")).unwrap(),
            "test"
        );
        assert!(target.join("test-a1b2/context.md").exists());
    }
}
