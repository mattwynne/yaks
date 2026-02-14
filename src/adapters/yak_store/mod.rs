// Storage adapters - implementations for different storage backends

pub mod directory;
#[cfg(any(test, feature = "test-support"))]
pub mod memory;

pub use directory::DirectoryStorage;
#[cfg(any(test, feature = "test-support"))]
pub use memory::InMemoryStorage;

#[cfg(test)]
mod contract_tests;

#[cfg(test)]
mod in_memory_contract {
    use super::contract_tests::yak_store_tests;
    yak_store_tests!((super::InMemoryStorage::new(), ()));
}

#[cfg(test)]
mod directory_contract {
    use super::contract_tests::yak_store_tests;
    use tempfile::TempDir;

    fn create_directory_store() -> (super::DirectoryStorage, TempDir) {
        let tmp = TempDir::new().unwrap();
        let storage = super::DirectoryStorage::from_path_unchecked(tmp.path().to_path_buf());
        (storage, tmp)
    }

    yak_store_tests!(create_directory_store());
}
