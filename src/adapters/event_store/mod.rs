pub mod git;
#[cfg(any(test, feature = "test-support"))]
pub mod memory;
pub mod migrate_v1_to_v2;
pub mod migrate_v2_to_v3;
pub mod migrate_v3_to_v4;
pub mod migration;
pub mod noop;

pub use git::GitEventStore;
#[cfg(any(test, feature = "test-support"))]
pub use memory::InMemoryEventStore;
pub use noop::NoOpEventStore;

#[cfg(test)]
mod contract_tests;

#[cfg(test)]
mod in_memory_contract {
    use super::contract_tests::event_store_tests;
    event_store_tests!((super::InMemoryEventStore::new(), ()));
}

#[cfg(test)]
mod git_contract {
    use super::contract_tests::event_store_tests;
    use git2::Repository;
    use tempfile::TempDir;

    fn create_git_store() -> (super::GitEventStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();
        (super::GitEventStore::from_repo(repo), tmp)
    }

    event_store_tests!(create_git_store());
}
