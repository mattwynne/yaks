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
                let yak_tree_oid = self.create_yak_tree(e.name.as_str(), "todo", "")?;
                let key = if e.id.as_str().is_empty() {
                    e.name.as_str()
                } else {
                    e.id.as_str()
                };
                let path = match &e.parent_id {
                    Some(parent) => format!("{}/{}", parent, key),
                    None => key.to_string(),
                };
                self.set_yak_in_root(current_tree, &path, Some(yak_tree_oid))
            }

            YakEvent::Removed(e) => self.set_yak_in_root(current_tree, e.id.as_str(), None),

            YakEvent::Moved(e) => {
                // Move yak subtree to new parent
                // For now, just update the tree location
                let old_subtree_oid = self
                    .get_yak_subtree(current_tree, e.id.as_str())?
                    .map(|t| t.id());

                let intermediate = self.set_yak_in_root(current_tree, e.id.as_str(), None)?;
                let intermediate_tree = self.repo.find_tree(intermediate)?;

                // Place under new parent if specified
                let target = match &e.new_parent {
                    Some(parent) => format!("{}/{}", parent, e.id),
                    None => e.id.to_string(),
                };
                self.set_yak_in_root(Some(&intermediate_tree), &target, old_subtree_oid)
            }

            YakEvent::Renamed(e) => {
                // Update name file for renamed yak
                self.update_yak_file(current_tree, e.id.as_str(), "name", e.new_name.as_str())
            }

            YakEvent::ContextUpdated(e) => {
                self.update_yak_file(current_tree, e.id.as_str(), "context.md", &e.content)
            }

            YakEvent::StateUpdated(e) => {
                self.update_yak_file(current_tree, e.id.as_str(), "state", &e.state)
            }

            YakEvent::FieldUpdated(e) => {
                self.update_yak_file(current_tree, e.id.as_str(), &e.field_name, &e.content)
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

        // Clean the target directory before recreating from git.
        // Remove yak directories (both current and stale) and files
        // that will be recreated from the git tree.
        // Non-yak files (e.g. notes.txt) and non-yak directories
        // (e.g. .git) are preserved.
        if target.exists() {
            let tree_names: std::collections::HashSet<String> = tree
                .iter()
                .filter_map(|e| e.name().map(String::from))
                .collect();

            for entry in std::fs::read_dir(target)? {
                let entry = entry?;
                let path = entry.path();
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if path.is_dir() {
                    // Remove directories that are in the git tree (will
                    // be recreated) or that look like yak entries (have
                    // a state file — these are stale).
                    if tree_names.contains(&*name_str) || path.join("state").exists() {
                        std::fs::remove_dir_all(&path)?;
                    }
                } else if tree_names.contains(&*name_str) {
                    std::fs::remove_file(&path)?;
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
    use crate::domain::slug::{Name, YakId};
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
                name: Name::from("test"),
                id: YakId::from("test-a1b2"),
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
                name: Name::from("test"),
                id: YakId::from(""),
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
                name: Name::from("test"),
                id: YakId::from("test-a1b2"),
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
                name: Name::from("test"),
                id: YakId::from("test-a1b2"),
                parent_id: None,
            }))
            .unwrap();

        store
            .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                id: YakId::from("test-a1b2"),
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
                name: Name::from("parent"),
                id: YakId::from("parent-a1b2"),
                parent_id: None,
            }))
            .unwrap();

        // Add child under parent
        store
            .append(&YakEvent::Added(AddedEvent {
                name: Name::from("child"),
                id: YakId::from("child-c3d4"),
                parent_id: Some(YakId::from("parent-a1b2")),
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
    fn materialize_tree_removes_stale_directories() {
        let (_tmp, mut store) = setup_test_repo();

        // Add a yak keyed by id "my-yak" in the git tree
        store
            .append(&YakEvent::Added(AddedEvent {
                name: Name::from("my yak"),
                id: YakId::from("my-yak"),
                parent_id: None,
            }))
            .unwrap();

        // Pre-populate target with a stale directory (different name
        // from the git tree key, simulating an old-style directory)
        // plus a non-yak file that should be preserved.
        let target = _tmp.path().join("yaks");
        std::fs::create_dir_all(target.join("my yak")).unwrap();
        std::fs::write(target.join("my yak/state"), "todo").unwrap();
        std::fs::write(target.join("notes.txt"), "keep me").unwrap();

        store.materialize_tree(&target).unwrap();

        assert!(
            target.join("my-yak").is_dir(),
            "Expected 'my-yak' directory from git tree"
        );
        assert!(
            !target.join("my yak").exists(),
            "Stale 'my yak' directory should have been removed"
        );
        assert!(
            target.join("notes.txt").exists(),
            "Non-yak files should be preserved"
        );
    }

    #[test]
    fn materialize_tree_round_trips() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(AddedEvent {
                name: Name::from("test"),
                id: YakId::from("test-a1b2"),
                parent_id: None,
            }))
            .unwrap();

        store
            .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                id: YakId::from("test-a1b2"),
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

    #[test]
    fn reset_from_snapshot_builds_correct_tree() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let mut fields = HashMap::new();
        fields.insert("plan".to_string(), "step 1".to_string());

        let yak1 = Yak {
            id: YakId::from("yak1-a1b2"),
            name: Name::from("First Yak"),
            state: "todo".to_string(),
            context: Some("some context".to_string()),
            fields: fields.clone(),
            children: vec![],
        };

        let yak2 = Yak {
            id: YakId::from("yak2-c3d4"),
            name: Name::from("Second Yak"),
            state: "wip".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
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
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
        };

        let parent = Yak {
            id: YakId::from("parent-a1b2"),
            name: Name::from("Parent Yak"),
            state: "wip".to_string(),
            context: Some("parent context".to_string()),
            fields: HashMap::new(),
            children: vec![YakId::from("child-x1y2")],
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
            .append(&YakEvent::Added(AddedEvent {
                name: Name::from("initial"),
                id: YakId::from("initial-z9z9"),
                parent_id: None,
            }))
            .unwrap();

        let first_commit_oid = store.get_latest_commit().unwrap().unwrap().id();

        // Now call reset_from_snapshot
        let yak = Yak {
            id: YakId::from("snapshot-a1b2"),
            name: Name::from("Snapshot Yak"),
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
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
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
        };

        let yak2 = Yak {
            id: YakId::from("yak2-c3d4"),
            name: Name::from("Yak Two"),
            state: "wip".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
        };

        let yak3 = Yak {
            id: YakId::from("yak3-e5f6"),
            name: Name::from("Yak Three"),
            state: "done".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
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
    fn reset_from_snapshot_errors_on_missing_child() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let parent = Yak {
            id: YakId::from("parent-a1b2"),
            name: Name::from("Parent Yak"),
            state: "wip".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![YakId::from("missing-child-x1y2")], // child doesn't exist
        };

        let result = store.reset_from_snapshot(&[parent]);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("missing-child-x1y2"));
        assert!(err_msg.contains("parent-a1b2"));
    }
}
