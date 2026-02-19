use anyhow::Result;
use git2::{ObjectType, Repository};

use super::migration::{EventStoreLocation, Migration};

/// Migration that flattens nested yak tree structure.
///
/// In v3, child yaks were nested inside parent yak subtrees.
/// In v4, ALL yaks are stored flat at the root of the git tree,
/// with a `parent_id` blob expressing hierarchy.
pub struct MigrateV3ToV4;

impl MigrateV3ToV4 {
    /// Check if a tree entry is a yak subtree (has `state` or `context.md`).
    fn is_yak_subtree(_repo: &Repository, tree: &git2::Tree) -> bool {
        tree.get_name("state").is_some() || tree.get_name("context.md").is_some()
    }

    /// Read the `id` blob from a yak subtree. Falls back to entry_name if missing.
    fn read_id(repo: &Repository, subtree: &git2::Tree, entry_name: &str) -> Result<String> {
        if let Some(id_entry) = subtree.get_name("id") {
            let blob = repo.find_blob(id_entry.id())?;
            Ok(std::str::from_utf8(blob.content())?.trim().to_string())
        } else {
            Ok(entry_name.to_string())
        }
    }

    /// Recursively collect all yaks from a nested tree, recording parent_id
    /// from nesting structure. Each yak is returned as (id, subtree_oid, parent_id).
    fn collect_yaks_recursive(
        repo: &Repository,
        tree: &git2::Tree,
        parent_id: Option<&str>,
        collected: &mut Vec<(String, git2::Oid, Option<String>)>,
    ) -> Result<()> {
        for entry in tree.iter() {
            if entry.kind() != Some(ObjectType::Tree) {
                continue;
            }
            let entry_name = match entry.name() {
                Some(n) => n.to_string(),
                None => continue,
            };

            let subtree = repo.find_tree(entry.id())?;
            if !Self::is_yak_subtree(repo, &subtree) {
                continue;
            }

            let yak_id = Self::read_id(repo, &subtree, &entry_name)?;

            // Build a clean subtree without nested child yak subtrees
            let mut builder = repo.treebuilder(Some(&subtree))?;

            // Remove any child yak subtrees from this yak's tree
            let child_entries: Vec<String> = subtree
                .iter()
                .filter(|e| e.kind() == Some(ObjectType::Tree))
                .filter_map(|e| {
                    let name = e.name()?.to_string();
                    let child_tree = repo.find_tree(e.id()).ok()?;
                    if Self::is_yak_subtree(repo, &child_tree) {
                        Some(name)
                    } else {
                        None
                    }
                })
                .collect();

            for child_name in &child_entries {
                builder.remove(child_name)?;
            }

            // Add parent_id blob if this yak has a parent
            if let Some(pid) = parent_id {
                let parent_id_blob = repo.blob(pid.as_bytes())?;
                builder.insert("parent_id", parent_id_blob, 0o100644)?;
            }

            let clean_oid = builder.write()?;
            collected.push((yak_id.clone(), clean_oid, parent_id.map(|s| s.to_string())));

            // Recurse into child yaks
            Self::collect_yaks_recursive(repo, &subtree, Some(&yak_id), collected)?;
        }
        Ok(())
    }
}

impl Migration for MigrateV3ToV4 {
    fn source_version(&self) -> u32 {
        3
    }
    fn target_version(&self) -> u32 {
        4
    }
    fn migrate(&self, location: &EventStoreLocation) -> Result<()> {
        let oid = location.repo.refname_to_id(location.ref_name)?;
        let parent_commit = location.repo.find_commit(oid)?;
        let root_tree = parent_commit.tree()?;

        let mut yaks = Vec::new();
        Self::collect_yaks_recursive(location.repo, &root_tree, None, &mut yaks)?;

        if yaks.is_empty() {
            return Ok(());
        }

        // Check if any work is needed: nesting to flatten OR entry keys
        // that don't match their id (e.g., old-style names with spaces)
        let has_nested = yaks.iter().any(|(_, _, pid)| pid.is_some());
        let has_miskeyed = root_tree.iter().any(|entry| {
            if entry.kind() != Some(ObjectType::Tree) {
                return false;
            }
            let entry_name = match entry.name() {
                Some(n) => n,
                None => return false,
            };
            let subtree = match location.repo.find_tree(entry.id()) {
                Ok(t) => t,
                Err(_) => return false,
            };
            match Self::read_id(location.repo, &subtree, entry_name) {
                Ok(id) => id != entry_name,
                Err(_) => false,
            }
        });

        if !has_nested && !has_miskeyed {
            return Ok(());
        }

        let mut root_builder = location.repo.treebuilder(None)?;

        for entry in root_tree.iter() {
            if entry.kind() == Some(ObjectType::Blob) {
                let name = match entry.name() {
                    Some(n) => n,
                    None => continue,
                };
                root_builder.insert(name, entry.id(), 0o100644)?;
            }
        }

        for (yak_id, subtree_oid, _) in &yaks {
            root_builder.insert(yak_id, *subtree_oid, 0o040000)?;
        }

        let new_root_oid = root_builder.write()?;
        let new_root_tree = location.repo.find_tree(new_root_oid)?;

        let sig = location
            .repo
            .signature()
            .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;

        location.repo.commit(
            Some(location.ref_name),
            &sig,
            &sig,
            "Migration v3→v4: flatten nested yak tree structure",
            &new_root_tree,
            &[&parent_commit],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::event_store::migration::tests::{read_yak_blob, setup_test_repo};
    use crate::adapters::event_store::migration::EventStoreLocation;

    fn location_for(repo: &Repository) -> EventStoreLocation<'_> {
        EventStoreLocation {
            repo,
            ref_name: "refs/notes/yaks",
        }
    }

    /// Helper: create a v3 tree with two-level nesting (parent → child).
    fn create_v3_nested_two_level(repo: &Repository) {
        let state_blob = repo.blob(b"todo").unwrap();
        let context_blob = repo.blob(b"").unwrap();
        let child_name = repo.blob(b"child").unwrap();
        let child_id = repo.blob(b"child-c3d4").unwrap();

        let mut child_builder = repo.treebuilder(None).unwrap();
        child_builder.insert("state", state_blob, 0o100644).unwrap();
        child_builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        child_builder.insert("name", child_name, 0o100644).unwrap();
        child_builder.insert("id", child_id, 0o100644).unwrap();
        let child_tree = child_builder.write().unwrap();

        let state_blob2 = repo.blob(b"wip").unwrap();
        let context_blob2 = repo.blob(b"parent context").unwrap();
        let parent_name = repo.blob(b"parent").unwrap();
        let parent_id = repo.blob(b"parent-a1b2").unwrap();

        let mut parent_builder = repo.treebuilder(None).unwrap();
        parent_builder
            .insert("state", state_blob2, 0o100644)
            .unwrap();
        parent_builder
            .insert("context.md", context_blob2, 0o100644)
            .unwrap();
        parent_builder
            .insert("name", parent_name, 0o100644)
            .unwrap();
        parent_builder.insert("id", parent_id, 0o100644).unwrap();
        parent_builder
            .insert("child-c3d4", child_tree, 0o040000)
            .unwrap();
        let parent_tree = parent_builder.write().unwrap();

        let version_blob = repo.blob(b"3").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("parent-a1b2", parent_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "v3 nested tree",
            &root_tree,
            &[],
        )
        .unwrap();
    }

    /// Helper: create a v3 tree with three-level nesting (grandparent → parent → child).
    fn create_v3_nested_three_level(repo: &Repository) {
        let state = repo.blob(b"todo").unwrap();
        let ctx = repo.blob(b"").unwrap();

        // grandchild
        let gc_name = repo.blob(b"child").unwrap();
        let gc_id = repo.blob(b"child-e5f6").unwrap();
        let mut gc_builder = repo.treebuilder(None).unwrap();
        gc_builder.insert("state", state, 0o100644).unwrap();
        gc_builder.insert("context.md", ctx, 0o100644).unwrap();
        gc_builder.insert("name", gc_name, 0o100644).unwrap();
        gc_builder.insert("id", gc_id, 0o100644).unwrap();
        let gc_tree = gc_builder.write().unwrap();

        // parent (middle level)
        let state2 = repo.blob(b"todo").unwrap();
        let ctx2 = repo.blob(b"").unwrap();
        let p_name = repo.blob(b"parent").unwrap();
        let p_id = repo.blob(b"parent-c3d4").unwrap();
        let mut p_builder = repo.treebuilder(None).unwrap();
        p_builder.insert("state", state2, 0o100644).unwrap();
        p_builder.insert("context.md", ctx2, 0o100644).unwrap();
        p_builder.insert("name", p_name, 0o100644).unwrap();
        p_builder.insert("id", p_id, 0o100644).unwrap();
        p_builder.insert("child-e5f6", gc_tree, 0o040000).unwrap();
        let p_tree = p_builder.write().unwrap();

        // grandparent (root level)
        let state3 = repo.blob(b"todo").unwrap();
        let ctx3 = repo.blob(b"").unwrap();
        let gp_name = repo.blob(b"grandparent").unwrap();
        let gp_id = repo.blob(b"grandparent-a1b2").unwrap();
        let mut gp_builder = repo.treebuilder(None).unwrap();
        gp_builder.insert("state", state3, 0o100644).unwrap();
        gp_builder.insert("context.md", ctx3, 0o100644).unwrap();
        gp_builder.insert("name", gp_name, 0o100644).unwrap();
        gp_builder.insert("id", gp_id, 0o100644).unwrap();
        gp_builder.insert("parent-c3d4", p_tree, 0o040000).unwrap();
        let gp_tree = gp_builder.write().unwrap();

        let version_blob = repo.blob(b"3").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("grandparent-a1b2", gp_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "v3 three-level nested tree",
            &root_tree,
            &[],
        )
        .unwrap();
    }

    /// Helper: create a v3 tree that is already flat (no nesting).
    fn create_v3_already_flat(repo: &Repository) {
        let state = repo.blob(b"todo").unwrap();
        let ctx = repo.blob(b"").unwrap();
        let name1 = repo.blob(b"alpha").unwrap();
        let id1 = repo.blob(b"alpha-a1b2").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state, 0o100644).unwrap();
        yak_builder.insert("context.md", ctx, 0o100644).unwrap();
        yak_builder.insert("name", name1, 0o100644).unwrap();
        yak_builder.insert("id", id1, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"3").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("alpha-a1b2", yak_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "v3 flat tree",
            &root_tree,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn v3_to_v4_flattens_two_level_nesting() {
        let (_tmp, repo) = setup_test_repo();
        create_v3_nested_two_level(&repo);

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        // Both yaks should be at root level
        assert!(
            read_yak_blob(&repo, "parent-a1b2", "name").is_some(),
            "parent should be at root"
        );
        assert!(
            read_yak_blob(&repo, "child-c3d4", "name").is_some(),
            "child should be at root"
        );

        // Child should have parent_id blob
        assert_eq!(
            read_yak_blob(&repo, "child-c3d4", "parent_id"),
            Some("parent-a1b2".to_string())
        );

        // Parent should NOT have parent_id blob
        assert_eq!(read_yak_blob(&repo, "parent-a1b2", "parent_id"), None);
    }

    #[test]
    fn v3_to_v4_flattens_three_level_nesting() {
        let (_tmp, repo) = setup_test_repo();
        create_v3_nested_three_level(&repo);

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        // All three yaks at root
        assert!(read_yak_blob(&repo, "grandparent-a1b2", "name").is_some());
        assert!(read_yak_blob(&repo, "parent-c3d4", "name").is_some());
        assert!(read_yak_blob(&repo, "child-e5f6", "name").is_some());

        // Check parent_id chain
        assert_eq!(read_yak_blob(&repo, "grandparent-a1b2", "parent_id"), None);
        assert_eq!(
            read_yak_blob(&repo, "parent-c3d4", "parent_id"),
            Some("grandparent-a1b2".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "child-e5f6", "parent_id"),
            Some("parent-c3d4".to_string())
        );
    }

    #[test]
    fn v3_to_v4_preserves_all_blobs() {
        let (_tmp, repo) = setup_test_repo();
        create_v3_nested_two_level(&repo);

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        // Parent blobs preserved
        assert_eq!(
            read_yak_blob(&repo, "parent-a1b2", "state"),
            Some("wip".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "parent-a1b2", "context.md"),
            Some("parent context".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "parent-a1b2", "name"),
            Some("parent".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "parent-a1b2", "id"),
            Some("parent-a1b2".to_string())
        );

        // Child blobs preserved
        assert_eq!(
            read_yak_blob(&repo, "child-c3d4", "state"),
            Some("todo".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "child-c3d4", "name"),
            Some("child".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "child-c3d4", "id"),
            Some("child-c3d4".to_string())
        );
    }

    // Mutant: line 59 `.filter(|e| e.kind() == Some(ObjectType::Tree))`
    // Changing == to != would skip tree entries, leaving child subtrees
    // inside the parent's flattened tree. This test verifies they're removed.
    #[test]
    fn v3_to_v4_removes_child_subtrees_from_parent() {
        let (_tmp, repo) = setup_test_repo();
        create_v3_nested_two_level(&repo);

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        // After flattening, the parent yak's tree should NOT contain
        // a "child-c3d4" subtree entry. The child is now at root level.
        let oid = repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = repo.find_commit(oid).unwrap();
        let root_tree = commit.tree().unwrap();
        let parent_entry = root_tree.get_name("parent-a1b2").unwrap();
        let parent_tree = repo.find_tree(parent_entry.id()).unwrap();

        assert!(
            parent_tree.get_name("child-c3d4").is_none(),
            "Parent's tree should not contain child subtree after flattening. \
             Found entries: {:?}",
            parent_tree
                .iter()
                .filter_map(|e| e.name().map(|n| n.to_string()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn v3_to_v4_no_parent_id_blob_for_root_yaks() {
        let (_tmp, repo) = setup_test_repo();
        create_v3_nested_two_level(&repo);

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        assert_eq!(
            read_yak_blob(&repo, "parent-a1b2", "parent_id"),
            None,
            "Root yaks should not have parent_id blob"
        );
    }

    #[test]
    fn v3_to_v4_noop_for_already_flat() {
        let (_tmp, repo) = setup_test_repo();
        create_v3_already_flat(&repo);

        let oid_before = repo.refname_to_id("refs/notes/yaks").unwrap();

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        // No new commit should have been created
        let oid_after = repo.refname_to_id("refs/notes/yaks").unwrap();
        assert_eq!(
            oid_before, oid_after,
            "No-op migration should not create a commit"
        );

        // Schema version unchanged (migration doesn't bump it)
        use crate::adapters::event_store::migration::read_schema_version;
        assert_eq!(read_schema_version(&location_for(&repo)).unwrap(), Some(3));
    }

    #[test]
    fn v3_to_v4_preserves_custom_fields() {
        let (_tmp, repo) = setup_test_repo();

        // Create a v3 tree with a custom field on a nested child
        let state = repo.blob(b"todo").unwrap();
        let ctx = repo.blob(b"").unwrap();
        let plan = repo.blob(b"step 1").unwrap();
        let child_name = repo.blob(b"child").unwrap();
        let child_id = repo.blob(b"child-c3d4").unwrap();

        let mut child_builder = repo.treebuilder(None).unwrap();
        child_builder.insert("state", state, 0o100644).unwrap();
        child_builder.insert("context.md", ctx, 0o100644).unwrap();
        child_builder.insert("name", child_name, 0o100644).unwrap();
        child_builder.insert("id", child_id, 0o100644).unwrap();
        child_builder.insert("plan", plan, 0o100644).unwrap();
        let child_tree = child_builder.write().unwrap();

        let state2 = repo.blob(b"todo").unwrap();
        let ctx2 = repo.blob(b"").unwrap();
        let parent_name = repo.blob(b"parent").unwrap();
        let parent_id = repo.blob(b"parent-a1b2").unwrap();

        let mut parent_builder = repo.treebuilder(None).unwrap();
        parent_builder.insert("state", state2, 0o100644).unwrap();
        parent_builder.insert("context.md", ctx2, 0o100644).unwrap();
        parent_builder
            .insert("name", parent_name, 0o100644)
            .unwrap();
        parent_builder.insert("id", parent_id, 0o100644).unwrap();
        parent_builder
            .insert("child-c3d4", child_tree, 0o040000)
            .unwrap();
        let parent_tree = parent_builder.write().unwrap();

        let version_blob = repo.blob(b"3").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("parent-a1b2", parent_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "v3 with custom field",
            &root_tree,
            &[],
        )
        .unwrap();

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        // Custom field preserved on flattened child
        assert_eq!(
            read_yak_blob(&repo, "child-c3d4", "plan"),
            Some("step 1".to_string())
        );
    }

    #[test]
    fn version_constants() {
        let m = MigrateV3ToV4;
        assert_eq!(m.source_version(), 3);
        assert_eq!(m.target_version(), 4);
    }

    fn make_tree_with_only_state(repo: &Repository) -> git2::Tree<'_> {
        let state_blob = repo.blob(b"todo").unwrap();
        let mut builder = repo.treebuilder(None).unwrap();
        builder.insert("state", state_blob, 0o100644).unwrap();
        let oid = builder.write().unwrap();
        repo.find_tree(oid).unwrap()
    }

    fn make_tree_with_only_context(repo: &Repository) -> git2::Tree<'_> {
        let context_blob = repo.blob(b"some notes").unwrap();
        let mut builder = repo.treebuilder(None).unwrap();
        builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        let oid = builder.write().unwrap();
        repo.find_tree(oid).unwrap()
    }

    fn make_empty_tree(repo: &Repository) -> git2::Tree<'_> {
        let builder = repo.treebuilder(None).unwrap();
        let oid = builder.write().unwrap();
        repo.find_tree(oid).unwrap()
    }

    #[test]
    fn is_yak_subtree_detects_tree_with_only_state() {
        let (_tmp, repo) = setup_test_repo();
        let tree = make_tree_with_only_state(&repo);
        assert!(
            MigrateV3ToV4::is_yak_subtree(&repo, &tree),
            "a tree with only 'state' should be detected as a yak subtree"
        );
    }

    #[test]
    fn is_yak_subtree_detects_tree_with_only_context() {
        let (_tmp, repo) = setup_test_repo();
        let tree = make_tree_with_only_context(&repo);
        assert!(
            MigrateV3ToV4::is_yak_subtree(&repo, &tree),
            "a tree with only 'context.md' should be detected as a yak subtree"
        );
    }

    #[test]
    fn is_yak_subtree_rejects_empty_tree() {
        let (_tmp, repo) = setup_test_repo();
        let tree = make_empty_tree(&repo);
        assert!(
            !MigrateV3ToV4::is_yak_subtree(&repo, &tree),
            "an empty tree should not be detected as a yak subtree"
        );
    }

    #[test]
    fn collect_yaks_skips_non_tree_entries() {
        let (_tmp, repo) = setup_test_repo();

        // Build a root with a blob and a non-yak tree (no state or context.md)
        let blob = repo.blob(b"data").unwrap();
        let mut non_yak_builder = repo.treebuilder(None).unwrap();
        non_yak_builder.insert("readme", blob, 0o100644).unwrap();
        let non_yak_tree = non_yak_builder.write().unwrap();

        let state_blob = repo.blob(b"todo").unwrap();
        let ctx_blob = repo.blob(b"").unwrap();
        let yak_name_blob = repo.blob(b"alpha").unwrap();
        let yak_id_blob = repo.blob(b"alpha-a1b2").unwrap();
        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder
            .insert("context.md", ctx_blob, 0o100644)
            .unwrap();
        yak_builder.insert("name", yak_name_blob, 0o100644).unwrap();
        yak_builder.insert("id", yak_id_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let schema_blob = repo.blob(b"3").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("not-a-yak", non_yak_tree, 0o040000)
            .unwrap();
        root_builder
            .insert("alpha-a1b2", yak_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", schema_blob, 0o100644)
            .unwrap();
        let root_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "root with mixed entries",
            &root_tree,
            &[],
        )
        .unwrap();

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        // Only the real yak should appear at root (non-yak tree skipped)
        assert!(
            read_yak_blob(&repo, "alpha-a1b2", "name").is_some(),
            "yak should remain after migration"
        );
    }

    #[test]
    fn v3_to_v4_renames_entry_keys_to_match_id() {
        let (_tmp, repo) = setup_test_repo();

        // Create a flat v3 tree where the tree entry key has spaces
        // but the id blob has the proper slugified version.
        // This happens when v2→v3 runs on old-style yaks.
        let state = repo.blob(b"todo").unwrap();
        let ctx = repo.blob(b"").unwrap();
        let name = repo.blob(b"fix the tests").unwrap();
        let id = repo.blob(b"fix-the-tests-a1b2").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state, 0o100644).unwrap();
        yak_builder.insert("context.md", ctx, 0o100644).unwrap();
        yak_builder.insert("name", name, 0o100644).unwrap();
        yak_builder.insert("id", id, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"3").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        // Key point: entry key is "fix the tests" (with spaces)
        root_builder
            .insert("fix the tests", yak_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "v3 flat tree with mismatched entry key",
            &root_tree,
            &[],
        )
        .unwrap();

        let migration = MigrateV3ToV4;
        migration.migrate(&location_for(&repo)).unwrap();

        // After migration, the yak should be keyed by its id, not the old name
        assert_eq!(
            read_yak_blob(&repo, "fix-the-tests-a1b2", "name"),
            Some("fix the tests".to_string()),
            "yak should be keyed by id after migration"
        );
        assert_eq!(
            read_yak_blob(&repo, "fix-the-tests-a1b2", "id"),
            Some("fix-the-tests-a1b2".to_string()),
        );
        // The old entry key should no longer exist
        assert_eq!(
            read_yak_blob(&repo, "fix the tests", "name"),
            None,
            "old space-containing entry key should be gone"
        );
    }
}
