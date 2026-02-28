/// Contract tests that must pass for all EventStore implementations.
/// Use the event_store_tests! macro to run against any implementation.
///
/// Note: The macro accepts an expression that returns `(impl EventStore, _guard)`.
/// The `_guard` keeps any resources (like TempDir) alive for the test duration.
/// For implementations that don't need a guard, pass `()`.
///
/// Content fields (FieldUpdatedEvent.content) may be empty when read back from
/// implementations that store content in trees rather than commit messages.
/// Tests only check event count, not content equality.
macro_rules! event_store_tests {
    ($create_store:expr) => {
        use crate::domain::event_metadata::EventMetadata;
        use crate::domain::ports::EventStore;
        use crate::domain::slug::{Name, YakId};
        use crate::domain::{AddedEvent, FieldUpdatedEvent, MovedEvent, RemovedEvent, YakEvent};

        #[test]
        fn appends_and_retrieves_single_event() {
            let (mut store, _guard) = $create_store;
            let event = YakEvent::Added(
                AddedEvent {
                    name: Name::from("foo"),
                    id: YakId::from("foo-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            );
            store.append(&event).unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 1);
        }

        #[test]
        fn appends_multiple_events() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("foo"),
                        id: YakId::from("foo-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("bar"),
                        id: YakId::from("bar-c3d4"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 2);
        }

        #[test]
        fn returns_events_in_chronological_order() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("first"),
                        id: YakId::from("first-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("second"),
                        id: YakId::from("second-c3d4"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all[0].yak_id(), "first-a1b2");
            assert_eq!(all[1].yak_id(), "second-c3d4");
        }

        #[test]
        fn filters_events_by_yak_id() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("foo"),
                        id: YakId::from("foo-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("bar"),
                        id: YakId::from("bar-c3d4"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: YakId::from("foo-a1b2"),
                        field_name: ".state".to_string(),
                        content: "wip".to_string(),
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            let foo_events = store.get_events("foo-a1b2").unwrap();
            assert_eq!(foo_events.len(), 2); // Added + FieldUpdated

            let bar_events = store.get_events("bar-c3d4").unwrap();
            assert_eq!(bar_events.len(), 1); // Added only

            let baz_events = store.get_events("baz").unwrap();
            assert_eq!(baz_events.len(), 0);
        }

        #[test]
        fn appended_events_have_event_id() {
            let (mut store, _guard) = $create_store;
            let event = YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            );
            store.append(&event).unwrap();
            let events = store.get_all_events().unwrap();
            let event_id = events[0].metadata().event_id.as_ref();
            assert!(
                event_id.is_some(),
                "event_id should be assigned by the store"
            );
            assert!(
                !event_id.unwrap().is_empty(),
                "event_id should not be empty"
            );
        }

        #[test]
        fn event_ids_are_unique() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("first"),
                        id: YakId::from("first-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("second"),
                        id: YakId::from("second-c3d4"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            let events = store.get_all_events().unwrap();
            let id1 = events[0].metadata().event_id.as_ref().unwrap();
            let id2 = events[1].metadata().event_id.as_ref().unwrap();
            assert_ne!(id1, id2, "event_ids should be unique across events");
        }

        #[test]
        fn append_is_idempotent_for_known_event_id() {
            let (mut store, _guard) = $create_store;
            let event = YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            );
            store.append(&event).unwrap();
            let events = store.get_all_events().unwrap();
            let event_with_id = events[0].clone();

            // Append same event again (has event_id from first append)
            store.append(&event_with_id).unwrap();
            let events = store.get_all_events().unwrap();
            assert_eq!(events.len(), 1, "duplicate should be skipped");
        }

        #[test]
        fn returns_empty_when_no_events() {
            let (store, _guard) = $create_store;
            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 0);
        }

        #[test]
        fn compaction_carries_snapshots() {
            let (mut store, _guard) = $create_store;
            // Add two yaks
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("foo"),
                        id: YakId::from("foo-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: YakId::from("foo-a1b2"),
                        field_name: ".state".to_string(),
                        content: "wip".to_string(),
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("bar"),
                        id: YakId::from("bar-c3d4"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            // Compact
            store.compact(EventMetadata::default_legacy()).unwrap();

            let events = store.get_all_events().unwrap();

            // Find the Compacted event
            let compacted = events
                .iter()
                .find(|e| matches!(e, YakEvent::Compacted(_, _)));
            assert!(compacted.is_some(), "Should have a Compacted event");

            if let YakEvent::Compacted(snapshots, _) = compacted.unwrap() {
                assert_eq!(snapshots.len(), 2, "Should have 2 snapshots");
                let foo = snapshots.iter().find(|s| s.id.as_str() == "foo-a1b2");
                assert!(foo.is_some(), "Should have snapshot for foo");
                assert_eq!(foo.unwrap().state, "wip");

                let bar = snapshots.iter().find(|s| s.id.as_str() == "bar-c3d4");
                assert!(bar.is_some(), "Should have snapshot for bar");
            }
        }

        #[test]
        fn events_after_compaction_are_preserved() {
            let (mut store, _guard) = $create_store;
            // Add a yak
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("foo"),
                        id: YakId::from("foo-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            // Compact
            store.compact(EventMetadata::default_legacy()).unwrap();

            // Add another yak after compaction
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("bar"),
                        id: YakId::from("bar-c3d4"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            let events = store.get_all_events().unwrap();

            // foo should be in the Compacted event's snapshots
            let compacted = events
                .iter()
                .find(|e| matches!(e, YakEvent::Compacted(_, _)));
            assert!(compacted.is_some(), "Should have a Compacted event");
            if let YakEvent::Compacted(snapshots, _) = compacted.unwrap() {
                assert!(
                    snapshots.iter().any(|s| s.id.as_str() == "foo-a1b2"),
                    "Compacted should have snapshot for foo"
                );
            }

            // bar should be a post-compaction Added event
            let added_ids: Vec<&str> = events
                .iter()
                .filter_map(|e| match e {
                    YakEvent::Added(a, _) => Some(a.id.as_str()),
                    _ => None,
                })
                .collect();
            assert!(
                added_ids.contains(&"bar-c3d4"),
                "Should have post-compaction Added for bar"
            );
        }

        #[test]
        fn latest_compaction_includes_all_yaks() {
            let (mut store, _guard) = $create_store;
            // Add foo
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("foo"),
                        id: YakId::from("foo-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            // First compaction
            store.compact(EventMetadata::default_legacy()).unwrap();

            // Add bar after first compaction
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("bar"),
                        id: YakId::from("bar-c3d4"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            // Second compaction (should include foo + bar)
            store.compact(EventMetadata::default_legacy()).unwrap();

            let events = store.get_all_events().unwrap();

            // Find the latest Compacted event (last one)
            let compacted_events: Vec<_> = events
                .iter()
                .filter(|e| matches!(e, YakEvent::Compacted(_, _)))
                .collect();
            let latest = compacted_events.last().unwrap();

            if let YakEvent::Compacted(snapshots, _) = latest {
                assert_eq!(
                    snapshots.len(),
                    2,
                    "Latest compaction should have both yaks"
                );
                assert!(
                    snapshots.iter().any(|s| s.id.as_str() == "foo-a1b2"),
                    "Should have foo in latest snapshot"
                );
                assert!(
                    snapshots.iter().any(|s| s.id.as_str() == "bar-c3d4"),
                    "Should have bar in latest snapshot"
                );
            }
        }

        #[test]
        fn compact_on_empty_store_fails() {
            let (mut store, _guard) = $create_store;
            let result = store.compact(EventMetadata::default_legacy());
            assert!(
                result.is_err(),
                "compact on empty store should fail, not create an empty snapshot"
            );
        }

        #[test]
        fn compact_creates_compacted_event_with_snapshots() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("foo"),
                        id: YakId::from("foo-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            store.compact(EventMetadata::default_legacy()).unwrap();

            let all = store.get_all_events().unwrap();
            let compacted = all.iter().find(|e| matches!(e, YakEvent::Compacted(_, _)));
            assert!(compacted.is_some(), "Should have a Compacted event");

            if let YakEvent::Compacted(snapshots, _) = compacted.unwrap() {
                assert_eq!(snapshots.len(), 1);
                assert_eq!(snapshots[0].id.as_str(), "foo-a1b2");
            }
        }

        #[test]
        fn double_compact_preserves_all_snapshots() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("foo"),
                        id: YakId::from("foo-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            store.compact(EventMetadata::default_legacy()).unwrap();
            store.compact(EventMetadata::default_legacy()).unwrap();

            let all = store.get_all_events().unwrap();
            // Find the latest Compacted event
            let compacted_events: Vec<_> = all
                .iter()
                .filter(|e| matches!(e, YakEvent::Compacted(_, _)))
                .collect();
            let latest = compacted_events.last().unwrap();

            if let YakEvent::Compacted(snapshots, _) = latest {
                assert_eq!(
                    snapshots.len(),
                    1,
                    "Latest compaction should still have foo"
                );
                assert_eq!(snapshots[0].id.as_str(), "foo-a1b2");
            }
        }

        #[test]
        fn roundtrips_all_event_types() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(
                    AddedEvent {
                        name: Name::from("test"),
                        id: YakId::from("test-a1b2"),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: YakId::from("test-a1b2"),
                        field_name: ".state".to_string(),
                        content: "wip".to_string(),
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::Moved(
                    MovedEvent {
                        id: YakId::from("test-a1b2"),
                        new_parent: Some(YakId::from("test2-c3d4")),
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: YakId::from("test2-c3d4"),
                        field_name: ".context.md".to_string(),
                        content: "some context".to_string(),
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: YakId::from("test2-c3d4"),
                        field_name: "notes".to_string(),
                        content: "stuff".to_string(),
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();
            store
                .append(&YakEvent::Removed(
                    RemovedEvent {
                        id: YakId::from("test2-c3d4"),
                    },
                    EventMetadata::default_legacy(),
                ))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 6);
        }
    };
}

pub(crate) use event_store_tests;
