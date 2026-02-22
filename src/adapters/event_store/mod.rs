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

use crate::domain::YakEvent;
use std::collections::HashSet;

/// Result of merging two event streams using CRDT-style set union.
pub(crate) struct MergeResult {
    /// All unique events, sorted by (timestamp, event_id) for convergence.
    pub events: Vec<YakEvent>,
    /// Number of events that exist in the peer but not locally (to pull).
    pub pulled: usize,
    /// Number of events that exist locally but not in the peer (to push).
    pub pushed: usize,
}

/// Merge two event streams by deduplicating on event_id, then sorting
/// deterministically by (timestamp, event_id). This ensures all peers
/// converge to the same ordered event list regardless of merge order.
pub(crate) fn merge_event_streams(
    local_events: &[YakEvent],
    peer_events: &[YakEvent],
) -> MergeResult {
    let mut all_events: Vec<YakEvent> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    for event in local_events.iter().chain(peer_events.iter()) {
        if let Some(id) = &event.metadata().event_id {
            if seen_ids.insert(id.clone()) {
                all_events.push(event.clone());
            }
        }
    }

    all_events.sort_by(|a, b| {
        a.metadata()
            .timestamp
            .as_epoch_secs()
            .cmp(&b.metadata().timestamp.as_epoch_secs())
            .then_with(|| {
                let id_a = a.metadata().event_id.as_deref().unwrap_or("");
                let id_b = b.metadata().event_id.as_deref().unwrap_or("");
                id_a.cmp(id_b)
            })
    });

    let local_ids: HashSet<String> = local_events
        .iter()
        .filter_map(|e| e.metadata().event_id.clone())
        .collect();
    let peer_ids: HashSet<String> = peer_events
        .iter()
        .filter_map(|e| e.metadata().event_id.clone())
        .collect();

    MergeResult {
        events: all_events,
        pulled: peer_ids.difference(&local_ids).count(),
        pushed: local_ids.difference(&peer_ids).count(),
    }
}

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
