use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::domain::event_metadata::EventMetadata;
use crate::domain::events::{AddedEvent, FieldUpdatedEvent};
use crate::domain::ports::{EventStore, EventStoreReader};
use crate::domain::slug::{Name, YakId};
use crate::domain::{Yak, YakEvent};

/// Replay events up to the latest Compacted event, rebuild state,
/// and synthesize Added/FieldUpdated events from that state.
/// Events after the latest Compacted are appended unchanged.
/// If no Compacted event exists, returns the original events as-is.
#[allow(clippy::cognitive_complexity)]
fn expand_compacted_events(events: &[YakEvent]) -> Result<Vec<YakEvent>> {
    // Find latest Compacted event index
    let compaction_idx = events.iter().enumerate().rev().find_map(|(i, e)| {
        if matches!(e, YakEvent::Compacted(_)) {
            Some(i)
        } else {
            None
        }
    });

    let Some(idx) = compaction_idx else {
        return Ok(events.to_vec());
    };

    // Extract the Compacted event's metadata
    let compaction_metadata = events[idx].metadata().clone();

    // Replay events before the compaction to build state
    let pre_compaction = &events[..idx];
    let post_compaction = &events[idx + 1..];

    // Build yak state from pre-compaction events
    struct YakState {
        name: Name,
        id: YakId,
        parent_id: Option<YakId>,
        state: String,
        context: Option<String>,
        fields: HashMap<String, String>,
        metadata: EventMetadata,
    }

    let mut yaks: HashMap<String, YakState> = HashMap::new();

    // Also replay any expanded events from earlier compactions
    // recursively (handles nested compactions)
    let expanded_pre = expand_compacted_events(pre_compaction)?;

    for event in &expanded_pre {
        match event {
            YakEvent::Added(e, m) => {
                yaks.insert(
                    e.id.as_str().to_string(),
                    YakState {
                        name: e.name.clone(),
                        id: e.id.clone(),
                        parent_id: e.parent_id.clone(),
                        state: "todo".to_string(),
                        context: None,
                        fields: HashMap::new(),
                        metadata: m.clone(),
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
                        "state" => yak.state = e.content.clone(),
                        "context.md" => yak.context = Some(e.content.clone()),
                        "name" => yak.name = Name::from(e.content.as_str()),
                        _ => {
                            yak.fields.insert(e.field_name.clone(), e.content.clone());
                        }
                    }
                }
            }
            YakEvent::Compacted(_) => {
                // Should not happen after recursive expansion, but
                // ignore if it does
            }
        }
    }

    // Synthesize events from the rebuilt state
    let mut result = Vec::new();

    // Sort yaks for deterministic output (parents before children)
    let mut sorted_yaks: Vec<&YakState> = yaks.values().collect();
    sorted_yaks.sort_by_key(|y| y.id.as_str().to_string());

    // Topological sort: emit parentless yaks first
    let mut emitted: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut remaining: Vec<&YakState> = sorted_yaks;
    let mut ordered: Vec<&YakState> = Vec::new();

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
                ordered.push(yak);
            } else {
                still_remaining.push(yak);
            }
        }

        remaining = still_remaining;
        if remaining.is_empty() || remaining.len() == before {
            ordered.extend(remaining);
            break;
        }
    }

    for yak in ordered {
        result.push(YakEvent::Added(
            AddedEvent {
                name: yak.name.clone(),
                id: yak.id.clone(),
                parent_id: yak.parent_id.clone(),
            },
            yak.metadata.clone(),
        ));

        if yak.state != "todo" {
            result.push(YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: yak.id.clone(),
                    field_name: "state".to_string(),
                    content: yak.state.clone(),
                },
                compaction_metadata.clone(),
            ));
        }

        if let Some(context) = &yak.context {
            if !context.is_empty() {
                result.push(YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: yak.id.clone(),
                        field_name: "context.md".to_string(),
                        content: context.clone(),
                    },
                    compaction_metadata.clone(),
                ));
            }
        }

        for (field_name, content) in &yak.fields {
            result.push(YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: yak.id.clone(),
                    field_name: field_name.clone(),
                    content: content.clone(),
                },
                compaction_metadata.clone(),
            ));
        }
    }

    // Include the Compacted marker event
    result.push(YakEvent::Compacted(compaction_metadata));

    // Append post-compaction events (excluding any Compacted events)
    for event in post_compaction {
        if !matches!(event, YakEvent::Compacted(_)) {
            result.push(event.clone());
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
        let events = self.events.lock().unwrap().clone();
        expand_compacted_events(&events)
    }

    fn compact(&mut self, metadata: crate::domain::event_metadata::EventMetadata) -> Result<()> {
        let events = self.events.lock().unwrap();
        if events.is_empty() {
            anyhow::bail!("Cannot compact an empty event store");
        }
        drop(events);
        let event = YakEvent::Compacted(metadata);
        self.append(&event)
    }

    fn reset_from_snapshot(&mut self, _yaks: &[Yak]) -> Result<usize> {
        Ok(0)
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

    #[test]
    fn test_reset_from_snapshot_returns_zero() {
        let mut store = InMemoryEventStore::new();
        let result = store.reset_from_snapshot(&[]).unwrap();

        assert_eq!(result, 0);
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
