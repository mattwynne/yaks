use anyhow::{bail, Result};
use git2::Repository;
use std::path::Path;

use super::migrate_v1_to_v2::MigrateV1ToV2;
use super::migrate_v2_to_v3::MigrateV2ToV3;
use super::migrate_v3_to_v4::MigrateV3ToV4;
use super::migrate_v4_to_v5::MigrateV4ToV5;
use super::migrate_v5_to_v6::MigrateV5ToV6;

/// The schema version this build of yx expects.
pub const CURRENT_SCHEMA_VERSION: u32 = 6;

/// A reference to a specific event store location in a git repository.
/// Bundles the repo and ref name to avoid threading them separately.
pub struct EventStoreLocation<'a> {
    pub repo: &'a Repository,
    pub ref_name: &'a str,
}

/// A migration that transforms the event store from one schema version to the next.
pub trait Migration {
    fn source_version(&self) -> u32;
    fn target_version(&self) -> u32;
    fn migrate(&self, location: &EventStoreLocation) -> Result<()>;
}

/// Manages schema versioning and migration for the git event store.
pub struct Migrator {
    expected_version: u32,
    migrations: Vec<Box<dyn Migration>>,
}

impl Migrator {
    pub fn new(expected_version: u32, migrations: Vec<Box<dyn Migration>>) -> Self {
        Self {
            expected_version,
            migrations,
        }
    }

    /// Create the default migrator with all registered migrations.
    pub fn for_current_version() -> Self {
        Self::new(
            CURRENT_SCHEMA_VERSION,
            vec![
                Box::new(MigrateV1ToV2),
                Box::new(MigrateV2ToV3),
                Box::new(MigrateV3ToV4),
                Box::new(MigrateV4ToV5),
                Box::new(MigrateV5ToV6),
            ],
        )
    }

    /// Run migration against a repo at the given path.
    /// Run migration against a repo at the given path.
    /// Returns true if migrations were performed (and the projection should be reset).
    pub fn run(&self, repo_path: &Path, ref_name: &str) -> Result<bool> {
        let repo = Repository::open(repo_path)
            .map_err(|_| anyhow::anyhow!("Error: not in a git repository"))?;
        let location = EventStoreLocation {
            repo: &repo,
            ref_name,
        };
        self.ensure_schema(&location)
    }

    /// Ensure the event store is at the expected schema version.
    /// Returns true if migrations were performed (and the projection should be reset).
    ///
    /// - Brand new repo (no ref): stamps expected version on first write.
    /// - Version matches: no-op.
    /// - Older version: runs migrations in order, then compacts.
    /// - Newer version: errors with "please update yx".
    pub fn ensure_schema(&self, location: &EventStoreLocation) -> Result<bool> {
        let current = match read_schema_version(location)? {
            Some(v) => v,
            None => return Ok(false), // Brand new repo — version stamped on first write
        };

        if current == self.expected_version {
            return Ok(false);
        }

        if current > self.expected_version {
            bail!(
                "Schema version {} is newer than this version of yx supports ({}). \
                 Please update yx.",
                current,
                self.expected_version
            );
        }

        // Run migrations from current to expected
        let mut version = current;
        let mut migrated = false;
        for migration in &self.migrations {
            if migration.source_version() == version {
                migration.migrate(location)?;
                version = migration.target_version();
                migrated = true;
            }
        }

        if migrated {
            // Compact after migration: create a Compacted commit so that
            // get_all_events() never walks past into pre-migration history.
            compact_ref(location, self.expected_version)?;
        } else {
            write_schema_version(location, self.expected_version)?;
        }
        Ok(migrated)
    }
}

/// Read the schema version from the event store tree at the given location.
/// Returns None if the ref doesn't exist (brand new repo).
/// Returns 1 if the ref exists but has no .schema-version blob.
pub fn read_schema_version(location: &EventStoreLocation) -> Result<Option<u32>> {
    let oid = match location.repo.refname_to_id(location.ref_name) {
        Ok(oid) => oid,
        Err(_) => return Ok(None),
    };

    let commit = location.repo.find_commit(oid)?;
    let tree = commit.tree()?;

    let entry_id = match tree.get_name(".schema-version") {
        Some(entry) => entry.id(),
        None => return Ok(Some(1)),
    };

    let blob = location.repo.find_blob(entry_id)?;
    let content = std::str::from_utf8(blob.content())?;
    let version: u32 = content.trim().parse()?;
    Ok(Some(version))
}

/// Write the schema version to .schema-version in the event store tree.
/// Creates a new commit on the location's ref with the updated tree.
pub fn write_schema_version(location: &EventStoreLocation, version: u32) -> Result<()> {
    let oid = location.repo.refname_to_id(location.ref_name)?;
    let parent = location.repo.find_commit(oid)?;
    let current_tree = parent.tree()?;

    let version_blob = location.repo.blob(version.to_string().as_bytes())?;
    let mut builder = location.repo.treebuilder(Some(&current_tree))?;
    builder.insert(".schema-version", version_blob, 0o100644)?;
    let new_tree_oid = builder.write()?;
    let new_tree = location.repo.find_tree(new_tree_oid)?;

    let sig = location
        .repo
        .signature()
        .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;

    location.repo.commit(
        Some(location.ref_name),
        &sig,
        &sig,
        &format!("Schema version: {}", version),
        &new_tree,
        &[&parent],
    )?;

    Ok(())
}

/// Compact the event store after migration: create a Compacted commit whose tree
/// is the current tip tree (with the schema version stamped). `get_all_events()`
/// stops at Compacted commits, so pre-migration history is never visited.
fn compact_ref(location: &EventStoreLocation, version: u32) -> Result<()> {
    let oid = location.repo.refname_to_id(location.ref_name)?;
    let parent = location.repo.find_commit(oid)?;
    let current_tree = parent.tree()?;

    // Stamp schema version on the tree
    let version_blob = location.repo.blob(version.to_string().as_bytes())?;
    let mut builder = location.repo.treebuilder(Some(&current_tree))?;
    builder.insert(".schema-version", version_blob, 0o100644)?;
    let new_tree_oid = builder.write()?;
    let new_tree = location.repo.find_tree(new_tree_oid)?;

    let sig = location
        .repo
        .signature()
        .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;

    location.repo.commit(
        Some(location.ref_name),
        &sig,
        &sig,
        &format!("Compacted\n\nEvent-Id: migration-to-v{}", version),
        &new_tree,
        &[&parent],
    )?;

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use tempfile::TempDir;

    pub fn setup_test_repo() -> (TempDir, Repository) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (tmp, repo)
    }

    /// Create a v1 event (Added) on refs/notes/yaks with no .schema-version.
    /// This duplicates the v1 format inline for the same reason as the
    /// Cucumber fixture — it's a frozen snapshot.
    pub fn create_v1_event(repo: &Repository, yak_name: &str) {
        let state_blob = repo.blob(b"todo").unwrap();
        let context_blob = repo.blob(b"").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder.insert(yak_name, yak_tree, 0o040000).unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();

        let sig = repo.signature().unwrap();
        let message = format!("Added: \"{}\"", yak_name);
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            &message,
            &root_tree,
            &[],
        )
        .unwrap();
    }

    /// Helper: read a blob from a yak subtree in refs/notes/yaks.
    pub fn read_yak_blob(repo: &Repository, yak_entry: &str, file_name: &str) -> Option<String> {
        let oid = repo.refname_to_id("refs/notes/yaks").ok()?;
        let commit = repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;
        let yak_entry = tree.get_name(yak_entry)?;
        let yak_tree = repo.find_tree(yak_entry.id()).ok()?;
        let blob_entry = yak_tree.get_name(file_name)?;
        let blob = repo.find_blob(blob_entry.id()).ok()?;
        Some(std::str::from_utf8(blob.content()).ok()?.to_string())
    }

    /// Helper: read a blob from a nested child yak subtree.
    pub fn read_child_yak_blob(
        repo: &Repository,
        parent_entry: &str,
        child_entry: &str,
        file_name: &str,
    ) -> Option<String> {
        let oid = repo.refname_to_id("refs/notes/yaks").ok()?;
        let commit = repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;
        let parent = tree.get_name(parent_entry)?;
        let parent_tree = repo.find_tree(parent.id()).ok()?;
        let child = parent_tree.get_name(child_entry)?;
        let child_tree = repo.find_tree(child.id()).ok()?;
        let blob_entry = child_tree.get_name(file_name)?;
        let blob = repo.find_blob(blob_entry.id()).ok()?;
        Some(std::str::from_utf8(blob.content()).ok()?.to_string())
    }

    #[test]
    fn read_schema_version_from_custom_ref() {
        let (_tmp, repo) = setup_test_repo();
        let state_blob = repo.blob(b"todo").unwrap();
        let context_blob = repo.blob(b"").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder.insert("test-yak", yak_tree, 0o040000).unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/custom/test"),
            &sig,
            &sig,
            "test event on custom ref",
            &root_tree,
            &[],
        )
        .unwrap();

        let location = EventStoreLocation {
            repo: &repo,
            ref_name: "refs/custom/test",
        };
        let version = read_schema_version(&location).unwrap();
        assert_eq!(version, Some(1));
    }

    #[test]
    fn write_schema_version_to_custom_ref() {
        let (_tmp, repo) = setup_test_repo();
        let state_blob = repo.blob(b"todo").unwrap();
        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder.insert("test-yak", yak_tree, 0o040000).unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/custom/test"),
            &sig,
            &sig,
            "test event",
            &root_tree,
            &[],
        )
        .unwrap();

        let location = EventStoreLocation {
            repo: &repo,
            ref_name: "refs/custom/test",
        };
        write_schema_version(&location, 3).unwrap();
        let version = read_schema_version(&location).unwrap();
        assert_eq!(version, Some(3));

        // Verify it didn't create refs/notes/yaks
        assert!(repo.refname_to_id("refs/notes/yaks").is_err());
    }

    fn location_for<'a>(repo: &'a Repository) -> EventStoreLocation<'a> {
        EventStoreLocation {
            repo,
            ref_name: "refs/notes/yaks",
        }
    }

    #[test]
    fn no_ref_means_brand_new_repo() {
        let (_tmp, repo) = setup_test_repo();
        let version = read_schema_version(&location_for(&repo)).unwrap();
        assert_eq!(version, None);
    }

    #[test]
    fn no_schema_version_blob_means_v1() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");
        let version = read_schema_version(&location_for(&repo)).unwrap();
        assert_eq!(version, Some(1));
    }

    #[test]
    fn reads_explicit_schema_version() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");
        write_schema_version(&location_for(&repo), 2).unwrap();
        let version = read_schema_version(&location_for(&repo)).unwrap();
        assert_eq!(version, Some(2));
    }

    // -- Migrator tests --

    use std::sync::atomic::{AtomicU32, Ordering};

    struct NoopMigration {
        from: u32,
        to: u32,
        call_count: AtomicU32,
    }

    impl NoopMigration {
        fn new(from: u32, to: u32) -> Self {
            Self {
                from,
                to,
                call_count: AtomicU32::new(0),
            }
        }

        fn was_called(&self) -> bool {
            self.call_count.load(Ordering::Relaxed) > 0
        }
    }

    impl Migration for NoopMigration {
        fn source_version(&self) -> u32 {
            self.from
        }
        fn target_version(&self) -> u32 {
            self.to
        }
        fn migrate(&self, _location: &EventStoreLocation) -> Result<()> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn version_matches_is_noop() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");
        let migrator = Migrator::new(1, vec![]);
        migrator.ensure_schema(&location_for(&repo)).unwrap();
        // No error, no version change
        assert_eq!(read_schema_version(&location_for(&repo)).unwrap(), Some(1));
    }

    #[test]
    fn newer_version_errors() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");
        write_schema_version(&location_for(&repo), 3).unwrap();
        let migrator = Migrator::new(2, vec![]);
        let err = migrator.ensure_schema(&location_for(&repo)).unwrap_err();
        assert!(
            err.to_string().contains("Please update yx"),
            "Expected 'Please update yx' error, got: {}",
            err
        );
    }

    #[test]
    fn runs_pending_migrations_in_order() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");

        let m1 = std::sync::Arc::new(NoopMigration::new(1, 2));
        let m2 = std::sync::Arc::new(NoopMigration::new(2, 3));

        // Wrap in Arc-based Migration impl
        let migrator = Migrator::new(
            3,
            vec![
                Box::new(ArcMigration(m1.clone())),
                Box::new(ArcMigration(m2.clone())),
            ],
        );
        migrator.ensure_schema(&location_for(&repo)).unwrap();

        assert!(m1.was_called(), "Migration 1→2 should have run");
        assert!(m2.was_called(), "Migration 2→3 should have run");
        assert_eq!(read_schema_version(&location_for(&repo)).unwrap(), Some(3));
    }

    #[test]
    fn brand_new_repo_skips_migrations() {
        let (_tmp, repo) = setup_test_repo();
        // No refs/notes/yaks at all
        let m1 = std::sync::Arc::new(NoopMigration::new(1, 2));
        let migrator = Migrator::new(2, vec![Box::new(ArcMigration(m1.clone()))]);
        migrator.ensure_schema(&location_for(&repo)).unwrap();
        assert!(!m1.was_called(), "Should not run migrations on new repo");
    }

    /// Wrapper to use Arc<NoopMigration> as a Box<dyn Migration>
    struct ArcMigration(std::sync::Arc<NoopMigration>);

    impl Migration for ArcMigration {
        fn source_version(&self) -> u32 {
            self.0.source_version()
        }
        fn target_version(&self) -> u32 {
            self.0.target_version()
        }
        fn migrate(&self, location: &EventStoreLocation) -> Result<()> {
            self.0.migrate(location)
        }
    }

    // -- v2→v3 migration tests --

    /// Create a v2 tree with an old-style yak (no name, no id) and schema version 2.
    pub fn create_v2_tree_with_old_yak(repo: &Repository, yak_name: &str) {
        create_v1_event(repo, yak_name);
        write_schema_version(&location_for(repo), 2).unwrap();
    }

    /// Create a v2 tree with a new-style yak (has name, no id) and schema version 2.
    pub fn create_v2_tree_with_new_yak(repo: &Repository, entry_key: &str, display_name: &str) {
        let state_blob = repo.blob(b"todo").unwrap();
        let context_blob = repo.blob(b"").unwrap();
        let name_blob = repo.blob(display_name.as_bytes()).unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        yak_builder.insert("name", name_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder.insert(entry_key, yak_tree, 0o040000).unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Added new-style yak",
            &root_tree,
            &[],
        )
        .unwrap();
        write_schema_version(&location_for(repo), 2).unwrap();
    }

    #[test]
    fn v2_to_v3_adds_name_and_id_to_old_style_yak() {
        let (_tmp, repo) = setup_test_repo();
        create_v2_tree_with_old_yak(&repo, "my test yak");

        let migration = MigrateV2ToV3;
        migration.migrate(&location_for(&repo)).unwrap();

        // Name should be the tree entry name
        assert_eq!(
            read_yak_blob(&repo, "my test yak", "name"),
            Some("my test yak".to_string())
        );
        // Id should be a generated slug-based id
        let id = read_yak_blob(&repo, "my test yak", "id").unwrap();
        assert!(
            id.starts_with("my-test-yak-"),
            "Expected id starting with 'my-test-yak-', got '{}'",
            id
        );
        assert_eq!(id.len(), "my-test-yak-".len() + 4);
    }

    #[test]
    fn v2_to_v3_preserves_existing_name_and_id() {
        let (_tmp, repo) = setup_test_repo();

        // Create a yak with name AND id already present
        let state_blob = repo.blob(b"wip").unwrap();
        let context_blob = repo.blob(b"some context").unwrap();
        let name_blob = repo.blob(b"My Yak").unwrap();
        let id_blob = repo.blob(b"my-yak-a1b2").unwrap();

        let mut yak_builder = repo.treebuilder(None).unwrap();
        yak_builder.insert("state", state_blob, 0o100644).unwrap();
        yak_builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        yak_builder.insert("name", name_blob, 0o100644).unwrap();
        yak_builder.insert("id", id_blob, 0o100644).unwrap();
        let yak_tree = yak_builder.write().unwrap();

        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("my-yak-a1b2", yak_tree, 0o040000)
            .unwrap();
        let root_tree_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_tree_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Added complete yak",
            &root_tree,
            &[],
        )
        .unwrap();
        write_schema_version(&location_for(&repo), 2).unwrap();

        let migration = MigrateV2ToV3;
        migration.migrate(&location_for(&repo)).unwrap();

        // Should be unchanged
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", "name"),
            Some("My Yak".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", "id"),
            Some("my-yak-a1b2".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "my-yak-a1b2", "context.md"),
            Some("some context".to_string())
        );
    }

    #[test]
    fn v2_to_v3_adds_id_to_new_style_yak_missing_id() {
        let (_tmp, repo) = setup_test_repo();
        create_v2_tree_with_new_yak(&repo, "make-tea-x1y2", "Make tea");

        let migration = MigrateV2ToV3;
        migration.migrate(&location_for(&repo)).unwrap();

        // Name should be preserved
        assert_eq!(
            read_yak_blob(&repo, "make-tea-x1y2", "name"),
            Some("Make tea".to_string())
        );
        // Id should be the tree entry name (since name blob exists → new-style)
        assert_eq!(
            read_yak_blob(&repo, "make-tea-x1y2", "id"),
            Some("make-tea-x1y2".to_string())
        );
    }

    #[test]
    fn v2_to_v3_handles_nested_old_style_yaks() {
        let (_tmp, repo) = setup_test_repo();

        // Create a tree with old-style parent containing an old-style child
        let state_blob = repo.blob(b"todo").unwrap();
        let context_blob = repo.blob(b"").unwrap();

        // Build child subtree (old-style: no name, no id)
        let mut child_builder = repo.treebuilder(None).unwrap();
        child_builder.insert("state", state_blob, 0o100644).unwrap();
        child_builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        let child_tree = child_builder.write().unwrap();

        // Build parent subtree (old-style: no name, no id) with child nested
        let state_blob2 = repo.blob(b"wip").unwrap();
        let context_blob2 = repo.blob(b"parent context").unwrap();
        let mut parent_builder = repo.treebuilder(None).unwrap();
        parent_builder
            .insert("state", state_blob2, 0o100644)
            .unwrap();
        parent_builder
            .insert("context.md", context_blob2, 0o100644)
            .unwrap();
        parent_builder
            .insert("child yak", child_tree, 0o040000)
            .unwrap();
        let parent_tree = parent_builder.write().unwrap();

        // Build root tree
        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("parent yak", parent_tree, 0o040000)
            .unwrap();
        let root_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Added nested yaks",
            &root_tree,
            &[],
        )
        .unwrap();
        write_schema_version(&location_for(&repo), 2).unwrap();

        let migration = MigrateV2ToV3;
        migration.migrate(&location_for(&repo)).unwrap();

        // Parent should have name and id
        assert_eq!(
            read_yak_blob(&repo, "parent yak", "name"),
            Some("parent yak".to_string())
        );
        let parent_id = read_yak_blob(&repo, "parent yak", "id").unwrap();
        assert!(
            parent_id.starts_with("parent-yak-"),
            "Expected parent id starting with 'parent-yak-', got '{}'",
            parent_id
        );

        // Child should have name and id
        assert_eq!(
            read_child_yak_blob(&repo, "parent yak", "child yak", "name"),
            Some("child yak".to_string())
        );
        let child_id = read_child_yak_blob(&repo, "parent yak", "child yak", "id").unwrap();
        assert!(
            child_id.starts_with("child-yak-"),
            "Expected child id starting with 'child-yak-', got '{}'",
            child_id
        );

        // Parent's other blobs should be preserved
        assert_eq!(
            read_yak_blob(&repo, "parent yak", "state"),
            Some("wip".to_string())
        );
        assert_eq!(
            read_yak_blob(&repo, "parent yak", "context.md"),
            Some("parent context".to_string())
        );
    }

    #[test]
    fn v2_to_v3_handles_old_parent_with_new_child() {
        let (_tmp, repo) = setup_test_repo();

        // Build new-style child (has name, no id)
        let state_blob = repo.blob(b"todo").unwrap();
        let context_blob = repo.blob(b"").unwrap();
        let child_name_blob = repo.blob(b"Fix the bug").unwrap();

        let mut child_builder = repo.treebuilder(None).unwrap();
        child_builder.insert("state", state_blob, 0o100644).unwrap();
        child_builder
            .insert("context.md", context_blob, 0o100644)
            .unwrap();
        child_builder
            .insert("name", child_name_blob, 0o100644)
            .unwrap();
        let child_tree = child_builder.write().unwrap();

        // Build old-style parent (no name, no id) with new-style child
        let state_blob2 = repo.blob(b"todo").unwrap();
        let context_blob2 = repo.blob(b"").unwrap();
        let mut parent_builder = repo.treebuilder(None).unwrap();
        parent_builder
            .insert("state", state_blob2, 0o100644)
            .unwrap();
        parent_builder
            .insert("context.md", context_blob2, 0o100644)
            .unwrap();
        parent_builder
            .insert("fix-the-bug-x1y2", child_tree, 0o040000)
            .unwrap();
        let parent_tree = parent_builder.write().unwrap();

        let mut root_builder = repo.treebuilder(None).unwrap();
        root_builder
            .insert("old parent", parent_tree, 0o040000)
            .unwrap();
        let root_oid = root_builder.write().unwrap();
        let root_tree = repo.find_tree(root_oid).unwrap();

        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            "Mixed old/new yaks",
            &root_tree,
            &[],
        )
        .unwrap();
        write_schema_version(&location_for(&repo), 2).unwrap();

        let migration = MigrateV2ToV3;
        migration.migrate(&location_for(&repo)).unwrap();

        // Old parent gets name and generated id
        assert_eq!(
            read_yak_blob(&repo, "old parent", "name"),
            Some("old parent".to_string())
        );
        let parent_id = read_yak_blob(&repo, "old parent", "id").unwrap();
        assert!(parent_id.starts_with("old-parent-"));

        // New-style child gets id = tree entry name, keeps existing name
        assert_eq!(
            read_child_yak_blob(&repo, "old parent", "fix-the-bug-x1y2", "name"),
            Some("Fix the bug".to_string())
        );
        assert_eq!(
            read_child_yak_blob(&repo, "old parent", "fix-the-bug-x1y2", "id"),
            Some("fix-the-bug-x1y2".to_string())
        );
    }

    #[test]
    fn v2_to_v3_preserves_schema_version_blob() {
        let (_tmp, repo) = setup_test_repo();
        create_v2_tree_with_old_yak(&repo, "test-yak");

        let migration = MigrateV2ToV3;
        migration.migrate(&location_for(&repo)).unwrap();

        // .schema-version should still be readable (preserved in root tree)
        let version = read_schema_version(&location_for(&repo)).unwrap();
        assert_eq!(version, Some(2)); // Migration doesn't bump version itself
    }
}
