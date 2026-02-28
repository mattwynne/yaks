use anyhow::Result;
use git2::ObjectType;

use super::migration::{EventStoreLocation, Migration};

/// Field renames: bare name → dot-prefixed name
const RENAMES: &[(&str, &str)] = &[
    ("state", ".state"),
    ("context.md", ".context.md"),
    ("name", ".name"),
    ("id", ".id"),
    ("parent_id", ".parent_id"),
];

/// Migration that renames reserved fields to dot-prefixed names in every yak subtree.
///
/// In v5, reserved fields were stored with bare names (state, context.md, name, id, parent_id).
/// In v6, they use dot-prefixed names (.state, .context.md, .name, .id, .parent_id) so they
/// cannot collide with user-defined fields.
pub struct MigrateV5ToV6;

impl Migration for MigrateV5ToV6 {
    fn source_version(&self) -> u32 {
        5
    }
    fn target_version(&self) -> u32 {
        6
    }
    fn migrate(&self, location: &EventStoreLocation) -> Result<()> {
        let oid = location.repo.refname_to_id(location.ref_name)?;
        let parent_commit = location.repo.find_commit(oid)?;
        let root_tree = parent_commit.tree()?;

        // Check if any yak subtree has bare-name fields that need renaming
        let needs_migration = root_tree.iter().any(|entry| {
            if entry.kind() != Some(ObjectType::Tree) {
                return false;
            }
            let subtree = match location.repo.find_tree(entry.id()) {
                Ok(t) => t,
                Err(_) => return false,
            };
            // If any of the bare names exist, we need to migrate
            RENAMES
                .iter()
                .any(|(old, _)| subtree.get_name(old).is_some())
        });

        if !needs_migration {
            return Ok(());
        }

        // Rebuild the root tree, renaming bare-name blobs to dot-prefixed in each yak subtree
        let mut root_builder = location.repo.treebuilder(None)?;

        for entry in root_tree.iter() {
            let entry_name = match entry.name() {
                Some(n) => n,
                None => continue,
            };

            if entry.kind() == Some(ObjectType::Tree) {
                let subtree = location.repo.find_tree(entry.id())?;

                // Check if this subtree needs renaming
                let has_bare_names = RENAMES
                    .iter()
                    .any(|(old, _)| subtree.get_name(old).is_some());

                if has_bare_names {
                    // Rebuild the yak subtree with renamed fields
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
                    // Already migrated or no bare names — keep as-is
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
            "Migration v5→v6: rename reserved fields to dot-prefix",
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

    /// Create a v5 tree with a yak that has bare-name fields
    fn create_v5_tree(repo: &git2::Repository, yak_id: &str) {
        let state_blob = repo.blob(b"wip").unwrap();
        let context_blob = repo.blob(b"some context").unwrap();
        let name_blob = repo.blob(b"My Yak").unwrap();
        let id_blob = repo.blob(yak_id.as_bytes()).unwrap();
        let parent_id_blob = repo.blob(b"parent-a1b2").unwrap();
        let created_blob = repo
            .blob(br#"{"created_by":{"name":"Alice","email":"alice@example.com"},"created_at":1234567890}"#)
            .unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        yak_builder.insert("name", name_blob, 0o100644).unwrap();
        yak_builder.insert("id", id_blob, 0o100644).unwrap();
        yak_builder
            .insert("parent_id", parent_id_blob, 0o100644)
            .unwrap();
        yak_builder
            .insert(".created.json", created_blob, 0o100644)
            .unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"5").unwrap();
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
            "Added yak with bare-name fields",
            &root_tree,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn renames_bare_fields_to_dot_prefix() {
        let (_tmp, repo) = setup_test_repo();
        create_v5_tree(&repo, "my-yak-a1b2");

        let migration = MigrateV5ToV6;
        migration.migrate(&location_for(&repo)).unwrap();

        // Old bare names should be gone
        assert_eq!(read_yak_blob(&repo, "my-yak-a1b2", "state"), None);
        assert_eq!(read_yak_blob(&repo, "my-yak-a1b2", "context.md"), None);
        assert_eq!(read_yak_blob(&repo, "my-yak-a1b2", "name"), None);
        assert_eq!(read_yak_blob(&repo, "my-yak-a1b2", "id"), None);
        assert_eq!(read_yak_blob(&repo, "my-yak-a1b2", "parent_id"), None);

        // Dot-prefixed names should exist with correct content
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", ".state"),
            Some("wip".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", ".context.md"),
            Some("some context".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", ".name"),
            Some("My Yak".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", ".id"),
            Some("my-yak-a1b2".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", ".parent_id"),
            Some("parent-a1b2".to_string())
        );

        // .created.json should be unchanged (already dot-prefixed)
        let created = read_yak_blob(&repo, "my-yak-a1b2", ".created.json").unwrap();
        let json: serde_json::Value = serde_json::from_str(&created).unwrap();
        assert_eq!(json["created_by"]["name"], "Alice");
    }

    #[test]
    fn preserves_custom_fields() {
        let (_tmp, repo) = setup_test_repo();

        // Create a yak with both reserved and custom fields
        let state_blob = repo.blob(b"todo").unwrap();
        let name_blob = repo.blob(b"Test").unwrap();
        let notes_blob = repo.blob(b"my notes").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder.insert("name", name_blob, 0o100644).unwrap();
        yak_builder.insert("notes", notes_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"5").unwrap();
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
            "Added yak with custom field",
            &root_tree,
            &[],
        )
        .unwrap();

        let migration = MigrateV5ToV6;
        migration.migrate(&location_for(&repo)).unwrap();

        // Custom field should be preserved unchanged
        assert_eq!(
            read_yak_blob(&repo, "test-a1b2", "notes"),
            Some("my notes".to_string())
        );
        // Reserved fields renamed
        assert_eq!(
            read_yak_blob(&repo, "test-a1b2", ".state"),
            Some("todo".to_string())
        );
    }

    #[test]
    fn preserves_schema_version_blob() {
        let (_tmp, repo) = setup_test_repo();
        create_v5_tree(&repo, "my-yak-a1b2");

        let migration = MigrateV5ToV6;
        migration.migrate(&location_for(&repo)).unwrap();

        let version = read_schema_version(&location_for(&repo)).unwrap();
        assert_eq!(version, Some(5)); // Migration doesn't bump version itself
    }

    #[test]
    fn noop_when_already_dot_prefixed() {
        let (_tmp, repo) = setup_test_repo();

        // Create a yak with already dot-prefixed fields
        let state_blob = repo.blob(b"todo").unwrap();
        let name_blob = repo.blob(b"Test").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert(".state", state_blob, 0o100644).unwrap();
        yak_builder.insert(".name", name_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"5").unwrap();
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

        let migration = MigrateV5ToV6;
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

        let state1 = repo.blob(b"wip").unwrap();
        let name1 = repo.blob(b"Yak One").unwrap();
        let state2 = repo.blob(b"done").unwrap();
        let name2 = repo.blob(b"Yak Two").unwrap();
        let state3_dot = repo.blob(b"todo").unwrap(); // already dot-prefixed

        // Yak 1 with bare names
        let mut y1 = repo.treebuilder(None).unwrap();
        y1.insert("state", state1, 0o100644).unwrap();
        y1.insert("name", name1, 0o100644).unwrap();
        let y1_tree = y1.write().unwrap();

        // Yak 2 with bare names
        let mut y2 = repo.treebuilder(None).unwrap();
        y2.insert("state", state2, 0o100644).unwrap();
        y2.insert("name", name2, 0o100644).unwrap();
        let y2_tree = y2.write().unwrap();

        // Yak 3 already dot-prefixed (no bare names)
        let mut y3 = repo.treebuilder(None).unwrap();
        y3.insert(".state", state3_dot, 0o100644).unwrap();
        let y3_tree = y3.write().unwrap();

        let version_blob = repo.blob(b"5").unwrap();
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

        let migration = MigrateV5ToV6;
        migration.migrate(&location_for(&repo)).unwrap();

        // Yak 1 and 2 should have dot-prefixed names
        assert_eq!(read_yak_blob(&repo, "yak-1", "state"), None);
        assert_eq!(
            read_yak_blob(&repo, "yak-1", ".state"),
            Some("wip".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "yak-1", ".name"),
            Some("Yak One".to_string())
        );

        assert_eq!(read_yak_blob(&repo, "yak-2", "state"), None);
        assert_eq!(
            read_yak_blob(&repo, "yak-2", ".state"),
            Some("done".to_string())
        );

        // Yak 3 should be unchanged
        assert_eq!(
            read_yak_blob(&repo, "yak-3", ".state"),
            Some("todo".to_string())
        );
    }
}
