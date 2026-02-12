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
        use crate::domain::{
            AddedEvent, ContextUpdatedEvent, FieldUpdatedEvent, MovedEvent, RemovedEvent,
            StateUpdatedEvent, YakEvent,
        };
        use crate::ports::EventStore;

        #[test]
        fn appends_and_retrieves_single_event() {
            let (mut store, _guard) = $create_store;
            let event = YakEvent::Added(AddedEvent {
                name: "foo".to_string(),
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
                    name: "foo".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "bar".to_string(),
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
                    name: "first".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "second".to_string(),
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
                    name: "foo".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Added(AddedEvent {
                    name: "bar".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                    name: "foo".to_string(),
                    state: "wip".to_string(),
                }))
                .unwrap();

            let foo_events = store.get_events("foo").unwrap();
            assert_eq!(foo_events.len(), 2);

            let bar_events = store.get_events("bar").unwrap();
            assert_eq!(bar_events.len(), 1);

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
                    name: "test".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::StateUpdated(StateUpdatedEvent {
                    name: "test".to_string(),
                    state: "wip".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Moved(MovedEvent {
                    old_name: "test".to_string(),
                    new_name: "test2".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::ContextUpdated(ContextUpdatedEvent {
                    name: "test2".to_string(),
                    content: "some context".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::FieldUpdated(FieldUpdatedEvent {
                    name: "test2".to_string(),
                    field_name: "notes".to_string(),
                    content: "stuff".to_string(),
                }))
                .unwrap();
            store
                .append(&YakEvent::Removed(RemovedEvent {
                    name: "test2".to_string(),
                }))
                .unwrap();

            let all = store.get_all_events().unwrap();
            assert_eq!(all.len(), 6);
        }
    };
}

pub(crate) use event_store_tests;
