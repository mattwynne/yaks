use anyhow::Result;
use git2::ObjectType;

use super::migration::{EventStoreLocation, Migration};

/// Migration that renames `.metadata.json` to `.created.json` in every yak subtree.
///
/// In v4, creation metadata (created_by, created_at) was stored in `.metadata.json`.
/// In v5, it's renamed to `.created.json` for clarity — the file specifically holds
/// creation-time information, not general metadata.
pub struct MigrateV4ToV5;

impl Migration for MigrateV4ToV5 {
    fn source_version(&self) -> u32 {
        4
    }
    fn target_version(&self) -> u32 {
        5
    }
    fn migrate(&self, location: &EventStoreLocation) -> Result<()> {
        let oid = location.repo.refname_to_id(location.ref_name)?;
        let parent_commit = location.repo.find_commit(oid)?;
        let root_tree = parent_commit.tree()?;

        // Check if any yak subtree has .metadata.json
        let needs_migration = root_tree.iter().any(|entry| {
            if entry.kind() != Some(ObjectType::Tree) {
                return false;
            }
            let subtree = match location.repo.find_tree(entry.id()) {
                Ok(t) => t,
                Err(_) => return false,
            };
            let has_metadata = subtree.get_name(".metadata.json").is_some();
            has_metadata
        });

        if !needs_migration {
            return Ok(());
        }

        // Rebuild the root tree, renaming .metadata.json → .created.json in each yak subtree
        let mut root_builder = location.repo.treebuilder(None)?;

        for entry in root_tree.iter() {
            let entry_name = match entry.name() {
                Some(n) => n,
                None => continue,
            };

            if entry.kind() == Some(ObjectType::Tree) {
                let subtree = location.repo.find_tree(entry.id())?;
                let meta_oid = subtree.get_name(".metadata.json").map(|e| e.id());
                if let Some(oid) = meta_oid {
                    // Rebuild this yak subtree with renamed file
                    let mut yak_builder = location.repo.treebuilder(Some(&subtree))?;
                    yak_builder.remove(".metadata.json")?;
                    yak_builder.insert(".created.json", oid, 0o100644)?;
                    let new_subtree_oid = yak_builder.write()?;
                    root_builder.insert(entry_name, new_subtree_oid, 0o040000)?;
                } else {
                    // Yak subtree without .metadata.json — keep as-is
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
            "Migration v4→v5: rename .metadata.json to .created.json",
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

    /// Create a v4 tree with a yak that has .metadata.json
    fn create_v4_tree_with_metadata(repo: &git2::Repository, yak_id: &str) {
        let state_blob = repo.blob(b"todo").unwrap();
        let name_blob = repo.blob(b"My Yak").unwrap();
        let id_blob = repo.blob(yak_id.as_bytes()).unwrap();
        let metadata_blob = repo
            .blob(br#"{"created_by":{"name":"Alice","email":"alice@example.com"},"created_at":1234567890}"#)
            .unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder.insert("name", name_blob, 0o100644).unwrap();
        yak_builder.insert("id", id_blob, 0o100644).unwrap();
        yak_builder
            .insert(".metadata.json", metadata_blob, 0o100644)
            .unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"4").unwrap();
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
            "Added yak with .metadata.json",
            &root_tree,
            &[],
        )
        .unwrap();
    }

    #[test]
    fn renames_metadata_json_to_created_json() {
        let (_tmp, repo) = setup_test_repo();
        create_v4_tree_with_metadata(&repo, "my-yak-a1b2");

        let migration = MigrateV4ToV5;
        migration.migrate(&location_for(&repo)).unwrap();

        // .metadata.json should be gone
        assert_eq!(read_yak_blob(&repo, "my-yak-a1b2", ".metadata.json"), None);

        // .created.json should exist with the same content
        let content = read_yak_blob(&repo, "my-yak-a1b2", ".created.json").unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["created_by"]["name"], "Alice");
        assert_eq!(json["created_by"]["email"], "alice@example.com");
        assert_eq!(json["created_at"], 1234567890);
    }

    #[test]
    fn preserves_other_yak_fields() {
        let (_tmp, repo) = setup_test_repo();
        create_v4_tree_with_metadata(&repo, "my-yak-a1b2");

        let migration = MigrateV4ToV5;
        migration.migrate(&location_for(&repo)).unwrap();

        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", "state"),
            Some("todo".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", "name"),
            Some("My Yak".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", "id"),
            Some("my-yak-a1b2".to_string())
        );
    }

    #[test]
    fn preserves_schema_version_blob() {
        let (_tmp, repo) = setup_test_repo();
        create_v4_tree_with_metadata(&repo, "my-yak-a1b2");

        let migration = MigrateV4ToV5;
        migration.migrate(&location_for(&repo)).unwrap();

        let version = read_schema_version(&location_for(&repo)).unwrap();
        assert_eq!(version, Some(4)); // Migration doesn't bump version itself
    }

    #[test]
    fn noop_when_no_metadata_json_present() {
        let (_tmp, repo) = setup_test_repo();

        // Create a yak without .metadata.json
        let state_blob = repo.blob(b"todo").unwrap();
        let id_blob = repo.blob(b"my-yak-a1b2").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder.insert("id", id_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let version_blob = repo.blob(b"4").unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("my-yak-a1b2", yak_tree, 0o040000)
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
                "Added yak without metadata",
                &root_tree,
                &[],
            )
            .unwrap();

        let migration = MigrateV4ToV5;
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

        let state_blob = repo.blob(b"todo").unwrap();
        let meta1 = repo
            .blob(br#"{"created_by":{"name":"Alice","email":"a@b.com"},"created_at":100}"#)
            .unwrap();
        let meta2 = repo
            .blob(br#"{"created_by":{"name":"Bob","email":"b@c.com"},"created_at":200}"#)
            .unwrap();

        // Yak 1 with .metadata.json
        let mut y1 = repo.treebuilder(None).unwrap();
        y1.insert("state", state_blob, 0o100644).unwrap();
        y1.insert(".metadata.json", meta1, 0o100644).unwrap();
        let y1_tree = y1.write().unwrap();

        // Yak 2 with .metadata.json
        let state_blob2 = repo.blob(b"wip").unwrap();
        let mut y2 = repo.treebuilder(None).unwrap();
        y2.insert("state", state_blob2, 0o100644).unwrap();
        y2.insert(".metadata.json", meta2, 0o100644).unwrap();
        let y2_tree = y2.write().unwrap();

        // Yak 3 without .metadata.json
        let state_blob3 = repo.blob(b"done").unwrap();
        let mut y3 = repo.treebuilder(None).unwrap();
        y3.insert("state", state_blob3, 0o100644).unwrap();
        let y3_tree = y3.write().unwrap();

        let version_blob = repo.blob(b"4").unwrap();
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

        let migration = MigrateV4ToV5;
        migration.migrate(&location_for(&repo)).unwrap();

        // Yak 1 and 2 should have .created.json, not .metadata.json
        assert_eq!(read_yak_blob(&repo, "yak-1", ".metadata.json"), None);
        assert!(read_yak_blob(&repo, "yak-1", ".created.json").is_some());

        assert_eq!(read_yak_blob(&repo, "yak-2", ".metadata.json"), None);
        assert!(read_yak_blob(&repo, "yak-2", ".created.json").is_some());

        // Yak 3 should have neither
        assert_eq!(read_yak_blob(&repo, "yak-3", ".metadata.json"), None);
        assert_eq!(read_yak_blob(&repo, "yak-3", ".created.json"), None);

        // Verify content
        let c1: serde_json::Value =
            serde_json::from_str(&read_yak_blob(&repo, "yak-1", ".created.json").unwrap()).unwrap();
        assert_eq!(c1["created_by"]["name"], "Alice");

        let c2: serde_json::Value =
            serde_json::from_str(&read_yak_blob(&repo, "yak-2", ".created.json").unwrap()).unwrap();
        assert_eq!(c2["created_by"]["name"], "Bob");
    }
}
