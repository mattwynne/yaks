use anyhow::{bail, Result};
use git2::{ObjectType, Repository};
use std::path::Path;

use crate::domain::slug::{generate_id, YakId};

/// The schema version this build of yx expects.
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

/// A migration that transforms the event store from one schema version to the next.
pub trait Migration {
    fn source_version(&self) -> u32;
    fn target_version(&self) -> u32;
    fn migrate(&self, repo: &Repository) -> Result<()>;
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
            vec![Box::new(MigrateV1ToV2), Box::new(MigrateV2ToV3)],
        )
    }

    /// Run migration against a repo at the given path.
    pub fn run(&self, repo_path: &Path) -> Result<()> {
        let repo = Repository::open(repo_path)
            .map_err(|_| anyhow::anyhow!("Error: not in a git repository"))?;
        self.ensure_schema(&repo)
    }

    /// Ensure the event store is at the expected schema version.
    /// - Brand new repo (no refs/notes/yaks): stamps expected version on first write.
    /// - Version matches: no-op.
    /// - Older version: runs migrations in order, stamps new version.
    /// - Newer version: errors with "please update yx".
    pub fn ensure_schema(&self, repo: &Repository) -> Result<()> {
        let current = match read_schema_version(repo)? {
            Some(v) => v,
            None => return Ok(()), // Brand new repo — version stamped on first write
        };

        if current == self.expected_version {
            return Ok(());
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
        for migration in &self.migrations {
            if migration.source_version() == version {
                migration.migrate(repo)?;
                version = migration.target_version();
            }
        }

        write_schema_version(repo, self.expected_version)?;
        Ok(())
    }
}

/// Read the schema version from the event store tree in refs/notes/yaks.
/// Returns None if refs/notes/yaks doesn't exist (brand new repo).
/// Returns 1 if the ref exists but has no .schema-version blob.
pub fn read_schema_version(repo: &Repository) -> Result<Option<u32>> {
    let oid = match repo.refname_to_id("refs/notes/yaks") {
        Ok(oid) => oid,
        Err(_) => return Ok(None),
    };

    let commit = repo.find_commit(oid)?;
    let tree = commit.tree()?;

    let entry_id = match tree.get_name(".schema-version") {
        Some(entry) => entry.id(),
        None => return Ok(Some(1)),
    };

    let blob = repo.find_blob(entry_id)?;
    let content = std::str::from_utf8(blob.content())?;
    let version: u32 = content.trim().parse()?;
    Ok(Some(version))
}

/// Write the schema version to .schema-version in the refs/notes/yaks tree.
/// Creates a new commit on refs/notes/yaks with the updated tree.
pub fn write_schema_version(repo: &Repository, version: u32) -> Result<()> {
    let oid = repo.refname_to_id("refs/notes/yaks")?;
    let parent = repo.find_commit(oid)?;
    let current_tree = parent.tree()?;

    let version_blob = repo.blob(version.to_string().as_bytes())?;
    let mut builder = repo.treebuilder(Some(&current_tree))?;
    builder.insert(".schema-version", version_blob, 0o100644)?;
    let new_tree_oid = builder.write()?;
    let new_tree = repo.find_tree(new_tree_oid)?;

    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;

    repo.commit(
        Some("refs/notes/yaks"),
        &sig,
        &sig,
        &format!("Schema version: {}", version),
        &new_tree,
        &[&parent],
    )?;

    Ok(())
}

/// No-op migration from v1 to v2.
/// Placeholder — will be fleshed out when yak names/IDs/paths lands.
struct MigrateV1ToV2;

impl Migration for MigrateV1ToV2 {
    fn source_version(&self) -> u32 {
        1
    }
    fn target_version(&self) -> u32 {
        2
    }
    fn migrate(&self, _repo: &Repository) -> Result<()> {
        // No-op for now. Future: transform events for slug-based IDs.
        Ok(())
    }
}

/// Migration that adds missing `name` and `id` blobs to yak subtrees.
///
/// Old-style yaks (pre-identity refactor) only have `state` and `context.md`.
/// This migration adds:
/// - `name` blob (from tree entry name) for old-style yaks
/// - `id` blob (generated or from tree entry name) for all yaks missing it
struct MigrateV2ToV3;

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
    fn migrate(&self, repo: &Repository) -> Result<()> {
        let oid = repo.refname_to_id("refs/notes/yaks")?;
        let parent = repo.find_commit(oid)?;
        let root_tree = parent.tree()?;

        let mut root_builder = repo.treebuilder(Some(&root_tree))?;
        let mut modified = false;

        // Walk root-level entries looking for yak subtrees
        for i in 0..root_tree.len() {
            let entry = root_tree.get(i).unwrap();
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
            if let Some(new_oid) = Self::migrate_subtree(repo, &subtree, &entry_name, None)? {
                root_builder.insert(&entry_name, new_oid, 0o040000)?;
                modified = true;
            }
        }

        if modified {
            let new_root_oid = root_builder.write()?;
            let new_root_tree = repo.find_tree(new_root_oid)?;
            let sig = repo
                .signature()
                .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;
            repo.commit(
                Some("refs/notes/yaks"),
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
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, Repository) {
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
    fn create_v1_event(repo: &Repository, yak_name: &str) {
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

    #[test]
    fn no_ref_means_brand_new_repo() {
        let (_tmp, repo) = setup_test_repo();
        let version = read_schema_version(&repo).unwrap();
        assert_eq!(version, None);
    }

    #[test]
    fn no_schema_version_blob_means_v1() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");
        let version = read_schema_version(&repo).unwrap();
        assert_eq!(version, Some(1));
    }

    #[test]
    fn reads_explicit_schema_version() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");
        write_schema_version(&repo, 2).unwrap();
        let version = read_schema_version(&repo).unwrap();
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
        fn migrate(&self, _repo: &Repository) -> Result<()> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn version_matches_is_noop() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");
        let migrator = Migrator::new(1, vec![]);
        migrator.ensure_schema(&repo).unwrap();
        // No error, no version change
        assert_eq!(read_schema_version(&repo).unwrap(), Some(1));
    }

    #[test]
    fn newer_version_errors() {
        let (_tmp, repo) = setup_test_repo();
        create_v1_event(&repo, "test-yak");
        write_schema_version(&repo, 3).unwrap();
        let migrator = Migrator::new(2, vec![]);
        let err = migrator.ensure_schema(&repo).unwrap_err();
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
        migrator.ensure_schema(&repo).unwrap();

        assert!(m1.was_called(), "Migration 1→2 should have run");
        assert!(m2.was_called(), "Migration 2→3 should have run");
        assert_eq!(read_schema_version(&repo).unwrap(), Some(3));
    }

    #[test]
    fn brand_new_repo_skips_migrations() {
        let (_tmp, repo) = setup_test_repo();
        // No refs/notes/yaks at all
        let m1 = std::sync::Arc::new(NoopMigration::new(1, 2));
        let migrator = Migrator::new(2, vec![Box::new(ArcMigration(m1.clone()))]);
        migrator.ensure_schema(&repo).unwrap();
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
        fn migrate(&self, repo: &Repository) -> Result<()> {
            self.0.migrate(repo)
        }
    }

    // -- v2→v3 migration tests --

    /// Helper: read a blob from a yak subtree in refs/notes/yaks.
    fn read_yak_blob(repo: &Repository, yak_entry: &str, file_name: &str) -> Option<String> {
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
    fn read_child_yak_blob(
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

    /// Create a v2 tree with an old-style yak (no name, no id) and schema version 2.
    fn create_v2_tree_with_old_yak(repo: &Repository, yak_name: &str) {
        create_v1_event(repo, yak_name);
        write_schema_version(repo, 2).unwrap();
    }

    /// Create a v2 tree with a new-style yak (has name, no id) and schema version 2.
    fn create_v2_tree_with_new_yak(repo: &Repository, entry_key: &str, display_name: &str) {
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
        write_schema_version(repo, 2).unwrap();
    }

    #[test]
    fn v2_to_v3_adds_name_and_id_to_old_style_yak() {
        let (_tmp, repo) = setup_test_repo();
        create_v2_tree_with_old_yak(&repo, "my test yak");

        let migration = MigrateV2ToV3;
        migration.migrate(&repo).unwrap();

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
        write_schema_version(&repo, 2).unwrap();

        let migration = MigrateV2ToV3;
        migration.migrate(&repo).unwrap();

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
        migration.migrate(&repo).unwrap();

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
        write_schema_version(&repo, 2).unwrap();

        let migration = MigrateV2ToV3;
        migration.migrate(&repo).unwrap();

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
        write_schema_version(&repo, 2).unwrap();

        let migration = MigrateV2ToV3;
        migration.migrate(&repo).unwrap();

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
        migration.migrate(&repo).unwrap();

        // .schema-version should still be readable (preserved in root tree)
        let version = read_schema_version(&repo).unwrap();
        assert_eq!(version, Some(2)); // Migration doesn't bump version itself
    }
}
