use anyhow::Result;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::domain::ports::{EventStore, EventStoreReader};
use crate::domain::{Yak, YakEvent};

#[derive(Clone)]
pub struct InMemoryEventStore {
    events: Arc<Mutex<Vec<YakEvent>>>,
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn get_events(&self, name: &str) -> Result<Vec<YakEvent>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .iter()
            .filter(|e| e.yak_id() == name)
            .cloned()
            .collect())
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for InMemoryEventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()> {
        let mut events = self.events.lock().unwrap();
        if let Some(id) = &event.metadata().event_id {
            if events
                .iter()
                .any(|e| e.metadata().event_id.as_deref() == Some(id))
            {
                return Ok(());
            }
        }
        let event = if event.metadata().event_id.is_none() {
            let mut metadata = event.metadata().clone();
            metadata.event_id = Some(uuid::Uuid::new_v4().to_string());
            event.clone().with_metadata(metadata)
        } else {
            event.clone()
        };
        events.push(event);
        Ok(())
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        Ok(self.events.lock().unwrap().clone())
    }

    fn reset_from_snapshot(&mut self, _yaks: &[Yak]) -> Result<usize> {
        Ok(0)
    }

    fn sync(
        &mut self,
        peer: &mut dyn EventStore,
        bus: &mut crate::infrastructure::event_bus::EventBus,
        output: &dyn crate::domain::ports::DisplayPort,
    ) -> Result<()> {
        // Pull: get events from peer that local doesn't have
        let local_ids: HashSet<String> = self
            .events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| e.metadata().event_id.clone())
            .collect();

        let peer_events = peer.get_all_events()?;
        let mut pulled = 0usize;
        for event in &peer_events {
            if let Some(id) = &event.metadata().event_id {
                if !local_ids.contains(id) {
                    self.append(event)?;
                    bus.notify(event)?;
                    pulled += 1;
                }
            }
        }

        // Push: get events from local that peer doesn't have
        let peer_ids: HashSet<String> = peer_events
            .iter()
            .filter_map(|e| e.metadata().event_id.clone())
            .collect();

        let local_events = self.events.lock().unwrap().clone();
        let mut pushed = 0usize;
        for event in &local_events {
            if let Some(id) = &event.metadata().event_id {
                if !peer_ids.contains(id) {
                    peer.append(event)?;
                    pushed += 1;
                }
            }
        }

        output.info(&format!(
            "Pulled {} events, pushed {} events",
            pulled, pushed
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
        use crate::adapters::InMemoryDisplay;
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

        #[test]
        fn pulls_events_from_peer() {
            let mut local = InMemoryEventStore::new();
            let mut peer = InMemoryEventStore::new();
            let mut bus = EventBus::new();
            let output = InMemoryDisplay::new();

            // Peer has an event, local is empty
            peer.append(&make_event("foo", "foo-a1b2")).unwrap();

            local.sync(&mut peer, &mut bus, &output).unwrap();

            assert_eq!(all_events(&local).len(), 1);
        }

        #[test]
        fn pushes_events_to_peer() {
            let mut local = InMemoryEventStore::new();
            let mut peer = InMemoryEventStore::new();
            let mut bus = EventBus::new();
            let output = InMemoryDisplay::new();

            // Local has an event, peer is empty
            local.append(&make_event("foo", "foo-a1b2")).unwrap();

            local.sync(&mut peer, &mut bus, &output).unwrap();

            assert_eq!(all_events(&peer).len(), 1);
        }

        #[test]
        fn merges_both_sides() {
            let mut local = InMemoryEventStore::new();
            let mut peer = InMemoryEventStore::new();
            let mut bus = EventBus::new();
            let output = InMemoryDisplay::new();

            // Local has event A, peer has event B
            local.append(&make_event("aaa", "aaa-a1b2")).unwrap();
            peer.append(&make_event("bbb", "bbb-c3d4")).unwrap();

            local.sync(&mut peer, &mut bus, &output).unwrap();

            assert_eq!(all_events(&local).len(), 2);
            assert_eq!(all_events(&peer).len(), 2);
        }

        #[test]
        fn notifies_bus_for_pulled_events() {
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

            let mut local = InMemoryEventStore::new();
            let mut peer = InMemoryEventStore::new();
            let mut bus = EventBus::new();
            let output = InMemoryDisplay::new();

            let captured = Arc::new(Mutex::new(Vec::new()));
            bus.register(Box::new(TestListener {
                events: Arc::clone(&captured),
            }));

            // Peer has an event, local is empty
            peer.append(&make_event("foo", "foo-a1b2")).unwrap();

            local.sync(&mut peer, &mut bus, &output).unwrap();

            let notified = captured.lock().unwrap();
            assert_eq!(notified.len(), 1, "bus should be notified of pulled event");
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

            let mut local = InMemoryEventStore::new();
            let mut peer = InMemoryEventStore::new();
            let mut bus = EventBus::new();
            let output = InMemoryDisplay::new();

            let captured = Arc::new(Mutex::new(Vec::new()));
            bus.register(Box::new(TestListener {
                events: Arc::clone(&captured),
            }));

            // Local has an event, peer is empty
            local.append(&make_event("foo", "foo-a1b2")).unwrap();

            local.sync(&mut peer, &mut bus, &output).unwrap();

            let notified = captured.lock().unwrap();
            assert_eq!(
                notified.len(),
                0,
                "bus should NOT be notified for pushed events"
            );
        }

        #[test]
        fn noop_when_stores_are_identical() {
            let mut local = InMemoryEventStore::new();
            let mut peer = InMemoryEventStore::new();
            let mut bus = EventBus::new();
            let output = InMemoryDisplay::new();

            // Add event to local, read it back (now has event_id)
            local.append(&make_event("foo", "foo-a1b2")).unwrap();
            let event_with_id = all_events(&local)[0].clone();

            // Add the same event (with event_id) to peer
            peer.append(&event_with_id).unwrap();

            local.sync(&mut peer, &mut bus, &output).unwrap();

            assert_eq!(all_events(&local).len(), 1);
            assert_eq!(all_events(&peer).len(), 1);
        }
    }
}
