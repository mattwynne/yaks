use anyhow::Result;
use git2::ObjectType;

use super::migration::{EventStoreLocation, Migration};

/// Field renames: bare name → dot-prefixed name
const RENAMES: &[(&str, &str)] = &[("tags", ".tags")];

/// Migration that renames the `tags` field to `.tags` in every yak subtree.
///
/// In v6, the tags field was stored as a bare `tags` blob, inconsistent with
/// all other reserved fields which use dot-prefixed names. In v7, tags are
/// stored as `.tags` to match the convention.
pub struct MigrateV6ToV7;

impl Migration for MigrateV6ToV7 {
    fn source_version(&self) -> u32 {
        6
    }
    fn target_version(&self) -> u32 {
        7
    }
    fn migrate(&self, location: &EventStoreLocation) -> Result<()> {
        let oid = location.repo.refname_to_id(location.ref_name)?;
        let parent_commit = location.repo.find_commit(oid)?;
        let root_tree = parent_commit.tree()?;

        // Check if any yak subtree has a bare `tags` blob that needs renaming
        let needs_migration = root_tree.iter().any(|entry| {
            if entry.kind() != Some(ObjectType::Tree) {
                return false;
            }
            let subtree = match location.repo.find_tree(entry.id()) {
                Ok(t) => t,
                Err(_) => return false,
            };
            RENAMES
                .iter()
                .any(|(old, _)| subtree.get_name(old).is_some())
        });

        if !needs_migration {
            return Ok(());
        }

        // Rebuild the root tree, renaming `tags` to `.tags` in each yak subtree
        let mut root_builder = location.repo.treebuilder(None)?;

        for entry in root_tree.iter() {
            let entry_name = match entry.name() {
                Some(n) => n,
                None => continue,
            };

            if entry.kind() == Some(ObjectType::Tree) {
                let subtree = location.repo.find_tree(entry.id())?;

                let has_bare_names = RENAMES
                    .iter()
                    .any(|(old, _)| subtree.get_name(old).is_some());

                if has_bare_names {
                    let mut yak_builder = location.repo.treebuilder(Some(&subtree))?;
                    for (old_name, new_name) in RENAMES {
                        if let Some(blob_entry) = subtree.get_name(old_name) {
                            let blob_oid = blob_entry.id();
                            yak_builder.remove(old_name)?;
                            yak_builder.insert(new_name, blob_oid, 0o100644)?;
                        }
                    }
                    let new_subtree_oid = yak_builder.write()?;
                    root_builder.insert(entry_name, new_subtree_oid, 0o040000)?;
                } else {
                    root_builder.insert(entry_name, entry.id(), 0o040000)?;
                }
            } else {
                // Blob entries (e.g., .schema-version) — keep as-is
                root_builder.insert(entry_name, entry.id(), entry.filemode())?;
            }
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
            "Migration v6→v7: rename tags to .tags",
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
    use crate::adapters::event_store::migration::{read_schema_version, EventStoreLocation};

    fn location_for(repo: &git2::Repository) -> EventStoreLocation<'_> {
        EventStoreLocation {
            repo,
            ref_name: "refs/notes/yaks",
        }
    }

    /// Create a v6 tree with a yak that has a bare `tags` blob.
    fn create_v6_tree(repo: &git2::Repository, yak_id: &str) {
        let state_blob = repo.blob(b"wip").unwrap();
        let name_blob = repo.blob(b"My Yak").unwrap();
        let id_blob = repo.blob(yak_id.as_bytes()).unwrap();
        let tags_blob = repo.blob(b"v1\nurgent").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert(".state", state_blob, 0o100644).unwrap();
        yak_builder.insert(".name", name_blob, 0o100644).unwrap();
        yak_builder.insert(".id", id_blob, 0o100644).unwrap();
        yak_builder.insert("tags", tags_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"6").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder.insert(yak_id, yak_tree, 0o040000).unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Added yak with bare tags field",
            &root_tree,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn renames_tags_to_dot_tags() {
        let (_tmp, repo) = setup_test_repo();
        create_v6_tree(&repo, "my-yak-a1b2");

        let migration = MigrateV6ToV7;
        migration.migrate(&location_for(&repo)).unwrap();

        // Old bare name should be gone
        assert_eq!(read_yak_blob(&repo, "my-yak-a1b2", "tags"), None);

        // Dot-prefixed name should exist with correct content
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", ".tags"),
            Some("v1\nurgent".to_string())
        );

        // Other fields should be unchanged
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", ".state"),
            Some("wip".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", ".name"),
            Some("My Yak".to_string())
        );
    }

    #[test]
    fn preserves_custom_fields() {
        let (_tmp, repo) = setup_test_repo();

        let state_blob = repo.blob(b"todo").unwrap();
        let name_blob = repo.blob(b"Test").unwrap();
        let tags_blob = repo.blob(b"v1").unwrap();
        let notes_blob = repo.blob(b"my notes").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert(".state", state_blob, 0o100644).unwrap();
        yak_builder.insert(".name", name_blob, 0o100644).unwrap();
        yak_builder.insert("tags", tags_blob, 0o100644).unwrap();
        yak_builder.insert("notes", notes_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"6").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("test-a1b2", yak_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Added yak with custom field and tags",
            &root_tree,
            &[],
        )
        .unwrap();

        let migration = MigrateV6ToV7;
        migration.migrate(&location_for(&repo)).unwrap();

        // Custom field should be preserved unchanged
        assert_eq!(
            read_yak_blob(&repo, "test-a1b2", "notes"),
            Some("my notes".to_string())
        );
        // Tags renamed
        assert_eq!(read_yak_blob(&repo, "test-a1b2", "tags"), None);
        assert_eq!(
            read_yak_blob(&repo, "test-a1b2", ".tags"),
            Some("v1".to_string())
        );
    }

    #[test]
    fn preserves_schema_version_blob() {
        let (_tmp, repo) = setup_test_repo();
        create_v6_tree(&repo, "my-yak-a1b2");

        let migration = MigrateV6ToV7;
        migration.migrate(&location_for(&repo)).unwrap();

        let version = read_schema_version(&location_for(&repo)).unwrap();
        assert_eq!(version, Some(6)); // Migration doesn't bump version itself
    }

    #[test]
    fn noop_when_already_dot_tags() {
        let (_tmp, repo) = setup_test_repo();

        // Create a yak with already dot-prefixed .tags
        let state_blob = repo.blob(b"todo").unwrap();
        let tags_blob = repo.blob(b"v1").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert(".state", state_blob, 0o100644).unwrap();
        yak_builder.insert(".tags", tags_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"6").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("test-a1b2", yak_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();

        let sig = repo.signature().unwrap();
        let initial_commit = repo
            .commit(
                Some("refs/notes/yaks"),
                &sig,
                &sig,
                "Already migrated yak",
                &root_tree,
                &[],
            )
            .unwrap();

        let migration = MigrateV6ToV7;
        migration.migrate(&location_for(&repo)).unwrap();

        // Should not create a new commit
        let head = repo.refname_to_id("refs/notes/yaks").unwrap();
        assert_eq!(
            head, initial_commit,
            "No-op migration should not create a commit"
        );
    }

    #[test]
    fn noop_when_no_tags_field() {
        let (_tmp, repo) = setup_test_repo();

        // Create a yak with no tags at all
        let state_blob = repo.blob(b"todo").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert(".state", state_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"6").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("test-a1b2", yak_tree, 0o040000)
            .unwrap();
        root_builder
            .insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();

        let sig = repo.signature().unwrap();
        let initial_commit = repo
            .commit(
                Some("refs/notes/yaks"),
                &sig,
                &sig,
                "Yak without tags",
                &root_tree,
                &[],
            )
            .unwrap();

        let migration = MigrateV6ToV7;
        migration.migrate(&location_for(&repo)).unwrap();

        // Should not create a new commit
        let head = repo.refname_to_id("refs/notes/yaks").unwrap();
        assert_eq!(
            head, initial_commit,
            "No-op migration should not create a commit"
        );
    }

    #[test]
    fn handles_multiple_yaks() {
        let (_tmp, repo) = setup_test_repo();

        let tags1 = repo.blob(b"v1\nv2").unwrap();
        let tags2 = repo.blob(b"urgent").unwrap();
        let tags3_dot = repo.blob(b"already-migrated").unwrap();
        let state = repo.blob(b"todo").unwrap();

        // Yak 1 with bare `tags`
        let mut y1 = repo.treebuilder(None).unwrap();
        y1.insert(".state", state, 0o100644).unwrap();
        y1.insert("tags", tags1, 0o100644).unwrap();
        let y1_tree = y1.write().unwrap();

        // Yak 2 with bare `tags`
        let mut y2 = repo.treebuilder(None).unwrap();
        y2.insert(".state", state, 0o100644).unwrap();
        y2.insert("tags", tags2, 0o100644).unwrap();
        let y2_tree = y2.write().unwrap();

        // Yak 3 already dot-prefixed (no bare `tags`)
        let mut y3 = repo.treebuilder(None).unwrap();
        y3.insert(".state", state, 0o100644).unwrap();
        y3.insert(".tags", tags3_dot, 0o100644).unwrap();
        let y3_tree = y3.write().unwrap();

        let version_blob = repo.blob(b"6").unwrap();
        let mut root = repo.treebuilder(None).unwrap();
        root.insert("yak-1", y1_tree, 0o040000).unwrap();
        root.insert("yak-2", y2_tree, 0o040000).unwrap();
        root.insert("yak-3", y3_tree, 0o040000).unwrap();
        root.insert(".schema-version", version_blob, 0o100644)
            .unwrap();
        let root_oid = root.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Multiple yaks",
            &root_tree,
            &[],
        )
        .unwrap();

        let migration = MigrateV6ToV7;
        migration.migrate(&location_for(&repo)).unwrap();

        // Yak 1: bare tags gone, .tags present
        assert_eq!(read_yak_blob(&repo, "yak-1", "tags"), None);
        assert_eq!(
            read_yak_blob(&repo, "yak-1", ".tags"),
            Some("v1\nv2".to_string())
        );

        // Yak 2: bare tags gone, .tags present
        assert_eq!(read_yak_blob(&repo, "yak-2", "tags"), None);
        assert_eq!(
            read_yak_blob(&repo, "yak-2", ".tags"),
            Some("urgent".to_string())
        );

        // Yak 3: unchanged
        assert_eq!(
            read_yak_blob(&repo, "yak-3", ".tags"),
            Some("already-migrated".to_string())
        );
    }

    #[test]
    fn version_constants() {
        let migration = MigrateV6ToV7;
        assert_eq!(migration.source_version(), 6);
        assert_eq!(migration.target_version(), 7);
    }
}
