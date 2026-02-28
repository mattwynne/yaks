mod commit;
pub mod git;
#[cfg(any(test, feature = "test-support"))]
pub mod memory;
pub mod migrate_v1_to_v2;
pub mod migrate_v2_to_v3;
pub mod migrate_v3_to_v4;
pub mod migrate_v4_to_v5;
pub mod migrate_v5_to_v6;
pub mod migration;
pub mod noop;
mod sync;
mod tree;

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
    uuid::Uuid::now_v7().to_string()
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

    // If the merged stream contains a Compacted event, check for
    // events that pre-date it but aren't represented in its snapshot.
    // These events "missed the checkpoint" and must be replayed after
    // the Compacted event, not before it (where clear_all would wipe them).
    if let Some(compact_idx) = all_events
        .iter()
        .position(|e| matches!(e, YakEvent::Compacted(_, _)))
    {
        if let YakEvent::Compacted(ref snapshots, _) = all_events[compact_idx] {
            let snapshot_yak_ids: HashSet<&str> = snapshots.iter().map(|s| s.id.as_str()).collect();

            // Collect indices of events before Compacted that affect
            // yak IDs not in the snapshot (they'd be lost on replay).
            let orphan_indices: Vec<usize> = (0..compact_idx)
                .filter(|&i| {
                    let yak_id = all_events[i].yak_id();
                    !yak_id.is_empty() && !snapshot_yak_ids.contains(yak_id)
                })
                .collect();

            if !orphan_indices.is_empty() {
                // Extract orphans, then reinsert them just after Compacted
                let mut orphans: Vec<YakEvent> = orphan_indices
                    .iter()
                    .rev()
                    .map(|&i| all_events.remove(i))
                    .collect();
                orphans.reverse();

                // Compacted has shifted left by the number of removals
                let new_compact_idx = all_events
                    .iter()
                    .position(|e| matches!(e, YakEvent::Compacted(_, _)))
                    .unwrap();

                // Insert orphans right after Compacted
                for (offset, orphan) in orphans.into_iter().enumerate() {
                    all_events.insert(new_compact_idx + 1 + offset, orphan);
                }
            }
        }
    }

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
mod merge_event_streams_tests {
    use super::merge_event_streams;
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
    use crate::domain::events::AddedEvent;
    use crate::domain::slug::{Name, YakId};
    use crate::domain::yak_snapshot::YakSnapshot;
    use crate::domain::YakEvent;
    use std::collections::HashMap;

    fn make_added(name: &str, id: &str, timestamp: i64, event_id: &str) -> YakEvent {
        let mut m = EventMetadata::new(
            Author {
                name: "test".into(),
                email: "".into(),
            },
            Timestamp(timestamp),
        );
        m.event_id = Some(event_id.to_string());
        YakEvent::Added(
            AddedEvent {
                name: Name::from(name),
                id: YakId::from(id),
                parent_id: None,
            },
            m,
        )
    }

    fn make_compacted(snapshots: Vec<YakSnapshot>, timestamp: i64, event_id: &str) -> YakEvent {
        let mut m = EventMetadata::new(
            Author {
                name: "alice".into(),
                email: "".into(),
            },
            Timestamp(timestamp),
        );
        m.event_id = Some(event_id.to_string());
        YakEvent::Compacted(snapshots, m)
    }

    fn snapshot(name: &str, id: &str) -> YakSnapshot {
        YakSnapshot {
            id: YakId::from(id),
            name: Name::from(name),
            parent_id: None,
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            created_by: Author {
                name: "test".into(),
                email: "".into(),
            },
            created_at: Timestamp(0),
        }
    }

    #[test]
    fn events_missing_from_compacted_snapshot_are_placed_after_it() {
        // Alice has: shared yak A, then compacted (snapshot contains A)
        // Bob has: shared yak A, then added yak B at T=80 (before compaction)
        // After merge, B must appear AFTER Compacted so it's not wiped
        let shared = make_added("alpha", "alpha-a1b2", 50, "evt-shared");
        let bobs_event = make_added("beta", "beta-c3d4", 80, "evt-bob");
        let compacted = make_compacted(vec![snapshot("alpha", "alpha-a1b2")], 100, "evt-compact");

        let alice_events = vec![shared.clone(), compacted.clone()];
        let bob_events = vec![shared.clone(), bobs_event.clone()];

        let result = merge_event_streams(&alice_events, &bob_events);

        let event_ids: Vec<&str> = result
            .events
            .iter()
            .map(|e| e.metadata().event_id.as_deref().unwrap())
            .collect();

        // Bob's event must come AFTER compacted, not before it
        let compact_pos = event_ids
            .iter()
            .position(|id| *id == "evt-compact")
            .unwrap();
        let bob_pos = event_ids.iter().position(|id| *id == "evt-bob").unwrap();
        assert!(
            bob_pos > compact_pos,
            "Bob's event (pos {}) must come after Compacted (pos {}). Order: {:?}",
            bob_pos,
            compact_pos,
            event_ids
        );
    }

    #[test]
    fn events_already_in_compacted_snapshot_stay_before_it() {
        // Alice compacted with A in snapshot. Bob also has A.
        // A should stay before Compacted (normal sort order).
        let shared = make_added("alpha", "alpha-a1b2", 50, "evt-shared");
        let compacted = make_compacted(vec![snapshot("alpha", "alpha-a1b2")], 100, "evt-compact");

        let alice_events = vec![shared.clone(), compacted.clone()];
        let bob_events = vec![shared.clone()];

        let result = merge_event_streams(&alice_events, &bob_events);

        let event_ids: Vec<&str> = result
            .events
            .iter()
            .map(|e| e.metadata().event_id.as_deref().unwrap())
            .collect();

        assert_eq!(event_ids, vec!["evt-shared", "evt-compact"]);
    }

    #[test]
    fn events_after_compaction_timestamp_are_not_reordered() {
        // Bob's event at T=120 (after compaction at T=100) stays in place
        let compacted = make_compacted(vec![snapshot("alpha", "alpha-a1b2")], 100, "evt-compact");
        let bobs_event = make_added("beta", "beta-c3d4", 120, "evt-bob");

        let alice_events = vec![compacted.clone()];
        let bob_events = vec![bobs_event.clone()];

        let result = merge_event_streams(&alice_events, &bob_events);

        let event_ids: Vec<&str> = result
            .events
            .iter()
            .map(|e| e.metadata().event_id.as_deref().unwrap())
            .collect();

        assert_eq!(event_ids, vec!["evt-compact", "evt-bob"]);
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
