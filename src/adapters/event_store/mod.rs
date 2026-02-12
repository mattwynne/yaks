pub mod git;
pub mod memory;

pub use git::GitEventStore;
pub use memory::InMemoryEventStore;

#[cfg(test)]
mod contract_tests;

#[cfg(test)]
mod in_memory_contract {
    use super::contract_tests::event_store_tests;
    event_store_tests!((super::InMemoryEventStore::new(), ()));
}
