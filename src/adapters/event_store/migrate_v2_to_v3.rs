use anyhow::Result;
use git2::{ObjectType, Repository};

use crate::domain::slug::{generate_id, YakId};

use super::migration::{EventStoreLocation, Migration};

/// Migration that adds missing `name` and `id` blobs to yak subtrees.
///
/// Old-style yaks (pre-identity refactor) only have `state` and `context.md`.
/// This migration adds:
/// - `name` blob (from tree entry name) for old-style yaks
/// - `id` blob (generated or from tree entry name) for all yaks missing it
pub struct MigrateV2ToV3;

impl MigrateV2ToV3 {
    /// Check if a tree entry is a yak subtree (has `state` or `context.md`).
    fn is_yak_subtree(_repo: &Repository, tree: &git2::Tree) -> bool {
        tree.get_name("state").is_some() || tree.get_name("context.md").is_some()
    }

    /// Recursively migrate a yak subtree, adding missing `name` and `id` blobs.
    /// Also recurses into child yak subtrees.
    /// Returns the new tree OID if modifications were made, or None if unchanged.
    fn migrate_subtree(
        repo: &Repository,
        tree: &git2::Tree,
        entry_name: &str,
        parent_yak_id: Option<&YakId>,
    ) -> Result<Option<git2::Oid>> {
        let mut modified = false;
        let mut builder = repo.treebuilder(Some(tree))?;

        // Determine if this subtree needs name/id
        let has_name = tree.get_name("name").is_some();
        let has_id = tree.get_name("id").is_some();

        if !has_name {
            // Old-style yak: tree entry name IS the display name
            let name_blob = repo.blob(entry_name.as_bytes())?;
            builder.insert("name", name_blob, 0o100644)?;
            modified = true;
        }

        // Determine this yak's ID (needed for recursion into children)
        let this_yak_id = if has_id {
            // Read existing id blob
            let id_entry = tree.get_name("id").unwrap();
            let blob = repo.find_blob(id_entry.id())?;
            YakId::from(std::str::from_utf8(blob.content())?.to_string())
        } else if has_name {
            // New-style yak: tree entry name is the ID
            let id_value = entry_name.to_string();
            let id_blob = repo.blob(id_value.as_bytes())?;
            builder.insert("id", id_blob, 0o100644)?;
            modified = true;
            YakId::from(entry_name)
        } else {
            // Old-style yak: generate deterministic ID from name + parent
            let generated = generate_id(entry_name, parent_yak_id);
            let id_blob = repo.blob(generated.as_str().as_bytes())?;
            builder.insert("id", id_blob, 0o100644)?;
            modified = true;
            generated
        };

        // Recurse into child yak subtrees
        for i in 0..tree.len() {
            let entry = tree.get(i).unwrap();
            if entry.kind() != Some(ObjectType::Tree) {
                continue;
            }
            let child_name = match entry.name() {
                Some(n) => n.to_string(),
                None => continue,
            };
            let child_tree = repo.find_tree(entry.id())?;
            if Self::is_yak_subtree(repo, &child_tree) {
                if let Some(new_child_oid) =
                    Self::migrate_subtree(repo, &child_tree, &child_name, Some(&this_yak_id))?
                {
                    builder.insert(&child_name, new_child_oid, 0o040000)?;
                    modified = true;
                }
            }
        }

        if modified {
            Ok(Some(builder.write()?))
        } else {
            Ok(None)
        }
    }
}

impl Migration for MigrateV2ToV3 {
    fn source_version(&self) -> u32 {
        2
    }
    fn target_version(&self) -> u32 {
        3
    }
    fn migrate(&self, location: &EventStoreLocation) -> Result<()> {
        let oid = location.repo.refname_to_id(location.ref_name)?;
        let parent = location.repo.find_commit(oid)?;
        let root_tree = parent.tree()?;

        let mut root_builder = location.repo.treebuilder(Some(&root_tree))?;
        let mut modified = false;

        for i in 0..root_tree.len() {
            let entry = root_tree.get(i).unwrap();
            if entry.kind() != Some(ObjectType::Tree) {
                continue;
            }
            let entry_name = match entry.name() {
                Some(n) => n.to_string(),
                None => continue,
            };
            let subtree = location.repo.find_tree(entry.id())?;
            if !Self::is_yak_subtree(location.repo, &subtree) {
                continue;
            }
            if let Some(new_oid) =
                Self::migrate_subtree(location.repo, &subtree, &entry_name, None)?
            {
                root_builder.insert(&entry_name, new_oid, 0o040000)?;
                modified = true;
            }
        }

        if modified {
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
                "Migration v2→v3: add name and id to yak subtrees",
                &new_root_tree,
                &[&parent],
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::event_store::migration::tests::setup_test_repo;
    use crate::adapters::event_store::migration::Migration;

    #[test]
    fn version_constants() {
        let m = MigrateV2ToV3;
        assert_eq!(m.source_version(), 2);
        assert_eq!(m.target_version(), 3);
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
            MigrateV2ToV3::is_yak_subtree(&repo, &tree),
            "a tree with only 'state' should be detected as a yak subtree"
        );
    }

    #[test]
    fn is_yak_subtree_detects_tree_with_only_context() {
        let (_tmp, repo) = setup_test_repo();
        let tree = make_tree_with_only_context(&repo);
        assert!(
            MigrateV2ToV3::is_yak_subtree(&repo, &tree),
            "a tree with only 'context.md' should be detected as a yak subtree"
        );
    }

    #[test]
    fn is_yak_subtree_rejects_empty_tree() {
        let (_tmp, repo) = setup_test_repo();
        let tree = make_empty_tree(&repo);
        assert!(
            !MigrateV2ToV3::is_yak_subtree(&repo, &tree),
            "an empty tree should not be detected as a yak subtree"
        );
    }
}
