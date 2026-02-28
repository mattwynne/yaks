use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domain::ports::{EventStore, EventStoreReader};
use crate::domain::slug::Name;
use crate::domain::YakEvent;

#[allow(clippy::cognitive_complexity)]
fn build_snapshots_from_events(
    events: &[YakEvent],
) -> Result<Vec<crate::domain::yak_snapshot::YakSnapshot>> {
    use crate::domain::yak_snapshot::YakSnapshot;
    use std::collections::HashSet;

    let mut yaks: HashMap<String, YakSnapshot> = HashMap::new();

    for event in events {
        match event {
            YakEvent::Added(e, m) => {
                yaks.insert(
                    e.id.as_str().to_string(),
                    YakSnapshot {
                        id: e.id.clone(),
                        name: e.name.clone(),
                        parent_id: e.parent_id.clone(),
                        state: "todo".to_string(),
                        context: None,
                        fields: HashMap::new(),
                        created_by: m.author.clone(),
                        created_at: m.timestamp,
                    },
                );
            }
            YakEvent::Removed(e, _) => {
                yaks.remove(e.id.as_str());
            }
            YakEvent::Moved(e, _) => {
                if let Some(yak) = yaks.get_mut(e.id.as_str()) {
                    yak.parent_id = e.new_parent.clone();
                }
            }
            YakEvent::FieldUpdated(e, _) => {
                if let Some(yak) = yaks.get_mut(e.id.as_str()) {
                    match e.field_name.as_str() {
                        ".state" => yak.state = e.content.clone(),
                        ".context.md" => yak.context = Some(e.content.clone()),
                        ".name" => yak.name = Name::from(e.content.as_str()),
                        _ => {
                            yak.fields.insert(e.field_name.clone(), e.content.clone());
                        }
                    }
                }
            }
            YakEvent::Compacted(snapshots, _) => {
                yaks.clear();
                for snap in snapshots {
                    yaks.insert(snap.id.as_str().to_string(), snap.clone());
                }
            }
        }
    }

    // Topological sort: parents before children
    let mut result = Vec::new();
    let mut emitted: HashSet<String> = HashSet::new();
    let mut remaining: Vec<YakSnapshot> = yaks.into_values().collect();
    remaining.sort_by_key(|y| y.id.as_str().to_string());

    loop {
        let before = remaining.len();
        let mut still_remaining = Vec::new();
        for yak in remaining {
            let can_emit = match &yak.parent_id {
                None => true,
                Some(pid) => emitted.contains(pid.as_str()),
            };
            if can_emit {
                emitted.insert(yak.id.as_str().to_string());
                result.push(yak);
            } else {
                still_remaining.push(yak);
            }
        }
        remaining = still_remaining;
        if remaining.is_empty() || remaining.len() == before {
            result.extend(remaining);
            break;
        }
    }

    Ok(result)
}

#[derive(Clone)]
pub struct InMemoryEventStore {
    events: Arc<Mutex<Vec<YakEvent>>>,
    peer: Option<Arc<Mutex<Vec<YakEvent>>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(vec![])),
            peer: None,
        }
    }

    /// Create an event store that syncs with the given peer's events
    pub fn with_peer(peer: &InMemoryEventStore) -> Self {
        Self {
            events: Arc::new(Mutex::new(vec![])),
            peer: Some(Arc::clone(&peer.events)),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for InMemoryEventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()> {
        let event = super::ensure_event_id(event.clone());
        let event_id = event.metadata().event_id.as_ref().unwrap();

        let mut events = self.events.lock().unwrap();
        if events
            .iter()
            .any(|e| e.metadata().event_id.as_deref() == Some(event_id))
        {
            return Ok(());
        }
        events.push(event);
        Ok(())
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        Ok(self.events.lock().unwrap().clone())
    }

    fn wipe(&mut self) -> Result<()> {
        self.events.lock().unwrap().clear();
        Ok(())
    }

    fn compact(&mut self, metadata: crate::domain::event_metadata::EventMetadata) -> Result<()> {
        let events = self.events.lock().unwrap();
        if events.is_empty() {
            anyhow::bail!("Cannot compact an empty event store");
        }
        let snapshots = build_snapshots_from_events(&events)?;
        drop(events);
        let event = YakEvent::Compacted(snapshots, metadata);
        self.append(&event)
    }

    fn sync(
        &mut self,
        _bus: &mut crate::infrastructure::event_bus::EventBus,
        output: &dyn crate::domain::ports::DisplayPort,
    ) -> Result<()> {
        let peer_events_arc = self
            .peer
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Sync not configured"))?
            .clone();

        let local_events = self.events.lock().unwrap().clone();
        let peer_events = peer_events_arc.lock().unwrap().clone();

        let merge = super::merge_event_streams(&local_events, &peer_events);

        // Replace both sides with sorted merged list
        *self.events.lock().unwrap() = merge.events.clone();
        *peer_events_arc.lock().unwrap() = merge.events;

        output.info(&format!(
            "Pulled {} events, pushed {} events",
            merge.pulled, merge.pushed
        ));

        Ok(())
    }
}

impl EventStoreReader for InMemoryEventStore {
    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        EventStore::get_all_events(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::EventMetadata;
    use crate::domain::events::AddedEvent;
    use crate::domain::slug::{Name, YakId};

    #[test]
    fn compact_stores_snapshot_in_event() {
        let mut store = InMemoryEventStore::new();
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
                crate::domain::events::FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: ".state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store.compact(EventMetadata::default_legacy()).unwrap();

        // Check the raw stored events (not get_all_events which may transform)
        let raw = store.events.lock().unwrap();
        let compacted = raw.iter().find(|e| matches!(e, YakEvent::Compacted(_, _)));
        assert!(compacted.is_some(), "Should have a Compacted event");

        if let YakEvent::Compacted(snapshots, _) = compacted.unwrap() {
            assert_eq!(snapshots.len(), 1);
            assert_eq!(snapshots[0].id, YakId::from("test-a1b2"));
            assert_eq!(snapshots[0].state, "wip");
        }
    }

    #[test]
    fn test_in_memory_event_store() {
        let mut store = InMemoryEventStore::new();

        let event = YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );

        store.append(&event).unwrap();
        let events = EventStore::get_all_events(&store).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].yak_id(), "");
    }

    #[test]
    fn test_get_all_events_empty_store() {
        let store = InMemoryEventStore::new();
        let events = EventStore::get_all_events(&store).unwrap();

        assert_eq!(events.len(), 0);
        assert!(events.is_empty());
    }

    mod sync {
        use super::*;
        use crate::adapters::make_test_display;
        use crate::infrastructure::event_bus::EventBus;

        fn make_event(name: &str, id: &str) -> YakEvent {
            YakEvent::Added(
                AddedEvent {
                    name: Name::from(name),
                    id: YakId::from(id),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            )
        }

        fn all_events(store: &InMemoryEventStore) -> Vec<YakEvent> {
            crate::domain::ports::EventStore::get_all_events(store).unwrap()
        }

        /// Helper: read events from a raw Arc<Mutex<Vec<YakEvent>>>
        fn peer_event_count(peer: &InMemoryEventStore) -> usize {
            peer.events.lock().unwrap().len()
        }

        #[test]
        fn pulls_events_from_peer() {
            let mut origin = InMemoryEventStore::new();
            origin.append(&make_event("foo", "foo-a1b2")).unwrap();

            let mut local = InMemoryEventStore::with_peer(&origin);
            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local.sync(&mut bus, &output).unwrap();

            assert_eq!(all_events(&local).len(), 1);
        }

        #[test]
        fn pushes_events_to_peer() {
            let origin = InMemoryEventStore::new();
            let mut local = InMemoryEventStore::with_peer(&origin);
            local.append(&make_event("foo", "foo-a1b2")).unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local.sync(&mut bus, &output).unwrap();

            assert_eq!(peer_event_count(&origin), 1);
        }

        #[test]
        fn merges_both_sides() {
            let mut origin = InMemoryEventStore::new();
            origin.append(&make_event("bbb", "bbb-c3d4")).unwrap();

            let mut local = InMemoryEventStore::with_peer(&origin);
            local.append(&make_event("aaa", "aaa-a1b2")).unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local.sync(&mut bus, &output).unwrap();

            assert_eq!(all_events(&local).len(), 2);
            assert_eq!(peer_event_count(&origin), 2);
        }

        #[test]
        fn sync_does_not_notify_bus_directly() {
            use crate::domain::ports::EventListener;
            use std::sync::{Arc, Mutex};

            struct TestListener {
                events: Arc<Mutex<Vec<YakEvent>>>,
            }

            impl EventListener for TestListener {
                fn on_event(&mut self, event: &YakEvent) -> Result<()> {
                    self.events.lock().unwrap().push(event.clone());
                    Ok(())
                }
            }

            let mut origin = InMemoryEventStore::new();
            origin.append(&make_event("foo", "foo-a1b2")).unwrap();

            let mut local = InMemoryEventStore::with_peer(&origin);
            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            let captured = Arc::new(Mutex::new(Vec::new()));
            bus.register(Box::new(TestListener {
                events: Arc::clone(&captured),
            }));

            local.sync(&mut bus, &output).unwrap();

            let notified = captured.lock().unwrap();
            assert_eq!(
                notified.len(),
                0,
                "sync itself should not notify bus (Application::sync_events handles rebuild)"
            );
        }

        #[test]
        fn does_not_notify_bus_for_pushed_events() {
            use crate::domain::ports::EventListener;
            use std::sync::{Arc, Mutex};

            struct TestListener {
                events: Arc<Mutex<Vec<YakEvent>>>,
            }

            impl EventListener for TestListener {
                fn on_event(&mut self, event: &YakEvent) -> Result<()> {
                    self.events.lock().unwrap().push(event.clone());
                    Ok(())
                }
            }

            let origin = InMemoryEventStore::new();
            let mut local = InMemoryEventStore::with_peer(&origin);
            local.append(&make_event("foo", "foo-a1b2")).unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            let captured = Arc::new(Mutex::new(Vec::new()));
            bus.register(Box::new(TestListener {
                events: Arc::clone(&captured),
            }));

            local.sync(&mut bus, &output).unwrap();

            let notified = captured.lock().unwrap();
            assert_eq!(
                notified.len(),
                0,
                "bus should NOT be notified for pushed events"
            );
        }

        #[test]
        fn noop_when_stores_are_identical() {
            let mut origin = InMemoryEventStore::new();
            origin.append(&make_event("foo", "foo-a1b2")).unwrap();
            let event_with_id = all_events(&origin)[0].clone();

            let mut local = InMemoryEventStore::with_peer(&origin);
            // Add the same event (with event_id) to local
            local.append(&event_with_id).unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local.sync(&mut bus, &output).unwrap();

            assert_eq!(all_events(&local).len(), 1);
            assert_eq!(peer_event_count(&origin), 1);
        }

        #[test]
        fn both_sides_have_identical_event_order_after_sync() {
            use crate::domain::event_metadata::{Author, Timestamp};

            let origin = InMemoryEventStore::new();
            let mut alice = InMemoryEventStore::with_peer(&origin);
            let mut bob = InMemoryEventStore::with_peer(&origin);

            let alice_event = YakEvent::Added(
                AddedEvent {
                    name: Name::from("alice-yak"),
                    id: YakId::from("alice-yak-a1b2"),
                    parent_id: None,
                },
                {
                    let mut m = EventMetadata::new(
                        Author {
                            name: "alice".into(),
                            email: "".into(),
                        },
                        Timestamp(100),
                    );
                    m.event_id = Some("event-alice".to_string());
                    m
                },
            );
            alice.append(&alice_event).unwrap();

            let bob_event = YakEvent::Added(
                AddedEvent {
                    name: Name::from("bob-yak"),
                    id: YakId::from("bob-yak-c3d4"),
                    parent_id: None,
                },
                {
                    let mut m = EventMetadata::new(
                        Author {
                            name: "bob".into(),
                            email: "".into(),
                        },
                        Timestamp(200),
                    );
                    m.event_id = Some("event-bob".to_string());
                    m
                },
            );
            bob.append(&bob_event).unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            alice.sync(&mut bus, &output).unwrap();
            bob.sync(&mut bus, &output).unwrap();
            alice.sync(&mut bus, &output).unwrap(); // pick up bob's event via origin

            let alice_ids: Vec<_> = all_events(&alice)
                .iter()
                .map(|e| e.metadata().event_id.clone().unwrap())
                .collect();
            let bob_ids: Vec<_> = all_events(&bob)
                .iter()
                .map(|e| e.metadata().event_id.clone().unwrap())
                .collect();

            assert_eq!(
                alice_ids, bob_ids,
                "Both sides should have identical event order"
            );
            assert_eq!(
                alice_ids,
                vec!["event-alice", "event-bob"],
                "Should be sorted by timestamp"
            );
        }

        #[test]
        fn same_timestamp_uses_event_id_as_tiebreaker() {
            use crate::domain::event_metadata::{Author, Timestamp};

            let mut origin = InMemoryEventStore::new();
            let mut local = InMemoryEventStore::with_peer(&origin);

            let event_z = YakEvent::Added(
                AddedEvent {
                    name: Name::from("aaa"),
                    id: YakId::from("aaa-a1b2"),
                    parent_id: None,
                },
                {
                    let mut m = EventMetadata::new(
                        Author {
                            name: "x".into(),
                            email: "".into(),
                        },
                        Timestamp(100),
                    );
                    m.event_id = Some("zzz-event".to_string());
                    m
                },
            );
            let event_a = YakEvent::Added(
                AddedEvent {
                    name: Name::from("bbb"),
                    id: YakId::from("bbb-c3d4"),
                    parent_id: None,
                },
                {
                    let mut m = EventMetadata::new(
                        Author {
                            name: "x".into(),
                            email: "".into(),
                        },
                        Timestamp(100),
                    );
                    m.event_id = Some("aaa-event".to_string());
                    m
                },
            );

            local.append(&event_z).unwrap();
            origin.append(&event_a).unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();
            local.sync(&mut bus, &output).unwrap();

            let ids: Vec<_> = all_events(&local)
                .iter()
                .map(|e| e.metadata().event_id.clone().unwrap())
                .collect();
            assert_eq!(ids, vec!["aaa-event", "zzz-event"]);
        }

        #[test]
        fn fails_when_no_peer_configured() {
            let mut local = InMemoryEventStore::new();
            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            let result = local.sync(&mut bus, &output);
            assert!(result.is_err());
            assert_eq!(result.unwrap_err().to_string(), "Sync not configured");
        }
    }
}
