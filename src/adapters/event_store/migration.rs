use anyhow::{bail, Result};
use git2::Repository;
use std::path::Path;

/// The schema version this build of yx expects.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

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
        Self::new(CURRENT_SCHEMA_VERSION, vec![Box::new(MigrateV1ToV2)])
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
}
