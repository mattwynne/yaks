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

/// Ensure an event has an event_id assigned. If the event already has one,
/// return it unchanged. Otherwise, generate a new UUID and return the
/// event with the ID set.
pub(crate) fn ensure_event_id(event: YakEvent) -> YakEvent {
    if event.metadata().event_id.is_some() {
        return event;
    }
    let mut metadata = event.metadata().clone();
    metadata.event_id = Some(generate_event_id());
    event.with_metadata(metadata)
}

pub(crate) fn generate_event_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

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
mod ensure_event_id_tests {
    use super::ensure_event_id;
    use crate::domain::event_metadata::EventMetadata;
    use crate::domain::events::AddedEvent;
    use crate::domain::slug::{Name, YakId};
    use crate::domain::YakEvent;

    #[test]
    fn assigns_event_id_when_missing() {
        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from("test-a1b2"),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );
        assert!(event.metadata().event_id.is_none());

        let event = ensure_event_id(event);
        assert!(event.metadata().event_id.is_some());
        assert!(!event.metadata().event_id.as_ref().unwrap().is_empty());
    }

    #[test]
    fn preserves_existing_event_id() {
        let mut metadata = EventMetadata::default_legacy();
        metadata.event_id = Some("existing-id".to_string());
        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from("test-a1b2"),
                parent_id: None,
            },
            metadata,
        );

        let event = ensure_event_id(event);
        assert_eq!(event.metadata().event_id.as_deref(), Some("existing-id"));
    }

    #[test]
    fn generates_unique_ids() {
        let make_event = || {
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            )
        };

        let e1 = ensure_event_id(make_event());
        let e2 = ensure_event_id(make_event());
        assert_ne!(
            e1.metadata().event_id,
            e2.metadata().event_id,
            "Each call should generate a unique ID"
        );
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
