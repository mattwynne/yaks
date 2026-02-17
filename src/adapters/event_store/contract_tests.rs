/// Contract tests that must pass for all EventStore implementations.
/// Use the event_store_tests! macro to run against any implementation.
///
/// Note: The macro accepts an expression that returns `(impl EventStore, _guard)`.
/// The `_guard` keeps any resources (like TempDir) alive for the test duration.
/// For implementations that don't need a guard, pass `()`.
///
/// Content fields (ContextUpdatedEvent.content, FieldUpdatedEvent.content) may
/// be empty when read back from implementations that store content in trees
/// rather than commit messages. Tests only check event count, not content equality.
macro_rules! event_store_tests {
    ($create_store:expr) => {
        use crate::domain::ports::EventStore;
        use crate::domain::slug::{Name, YakId};
        use crate::domain::{
            AddedEvent, ContextUpdatedEvent, FieldUpdatedEvent, MovedEvent, RemovedEvent,
            StateUpdatedEvent, YakEvent,
        };

        #[test]
        fn appends_and_retrieves_single_event() {
            let (mut store, _guard) = $create_store;
            let event = YakEvent::Added(AddedEvent {
                name: Name::from("foo"),
                id: YakId::from("foo-a1b2"),
                parent_id: None,
            });
            store.append(&event).unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 1);
        }

        #[test]
        fn appends_multiple_events() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: Name::from("foo"),
                    id: YakId::from("foo-a1b2"),
                    parent_id: None,
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: Name::from("bar"),
                    id: YakId::from("bar-c3d4"),
                    parent_id: None,
                }))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 2);
        }

        #[test]
        fn returns_events_in_chronological_order() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: Name::from("first"),
                    id: YakId::from("first-a1b2"),
                    parent_id: None,
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: Name::from("second"),
                    id: YakId::from("second-c3d4"),
                    parent_id: None,
                }))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all[0].yak_name(), "first");
            assert_eq!(all[1].yak_name(), "second");
        }

        #[test]
        fn filters_events_by_yak_name() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: Name::from("foo"),
                    id: YakId::from("foo-a1b2"),
                    parent_id: None,
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: Name::from("bar"),
                    id: YakId::from("bar-c3d4"),
                    parent_id: None,
                }))
                .unwrap();
            store
                .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                    id: YakId::from("foo-a1b2"),
                    state: "wip".to_string(),
                }))
                .unwrap();

            // Note: get_events filters by yak_name(), which returns
            // the Name for Added events but the ID for all others.
            // This is a known inconsistency (see "fix yak_name
            // method inconsistency on YakEvent" yak).
            let foo_events = store.get_events("foo-a1b2").unwrap();
            assert_eq!(foo_events.len(), 1); // StateUpdated only

            let bar_events = store.get_events("bar-c3d4").unwrap();
            assert_eq!(bar_events.len(), 0); // no non-Added events

            let baz_events = store.get_events("baz").unwrap();
            assert_eq!(baz_events.len(), 0);
        }

        #[test]
        fn returns_empty_when_no_events() {
            let (store, _guard) = $create_store;
            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 0);
        }

        #[test]
        fn roundtrips_all_event_types() {
            let (mut store, _guard) = $create_store;
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                }))
                .unwrap();
            store
                .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    state: "wip".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Moved(MovedEvent {
                    id: YakId::from("test-a1b2"),
                    new_parent: Some(YakId::from("test2-c3d4")),
                }))
                .unwrap();
            store
                .append(&YakEvent::ContextUpdated(ContextUpdatedEvent {
                    id: YakId::from("test2-c3d4"),
                    content: "some context".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::FieldUpdated(FieldUpdatedEvent {
                    id: YakId::from("test2-c3d4"),
                    field_name: "notes".to_string(),
                    content: "stuff".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Removed(RemovedEvent {
                    id: YakId::from("test2-c3d4"),
                }))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 6);
        }
    };
}

pub(crate) use event_store_tests;
