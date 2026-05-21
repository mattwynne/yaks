use anyhow::Result;
use std::collections::HashSet;
use std::time::Duration;

use crate::domain::events::BlockerSource;
use crate::domain::ports::{GlobalEventBus, GlobalEventSubscription};
use crate::domain::{EventMetadata, YakEvent, YakId, YakMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventWatchScope {
    All,
    Subtree(YakId),
}

pub struct EventWatcher<'a> {
    subscription: Box<dyn GlobalEventSubscription + 'a>,
    filter: EventSubtreeFilter,
}

impl<'a> EventWatcher<'a> {
    pub fn subscribe(
        bus: &'a mut dyn GlobalEventBus,
        scope: EventWatchScope,
        historical_events: Vec<YakEvent>,
    ) -> Result<Self> {
        Ok(Self {
            subscription: bus.subscribe_from_now()?,
            filter: EventSubtreeFilter::new(scope, historical_events),
        })
    }

    pub fn next_relevant_batch(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<Option<Vec<YakEvent>>> {
        let Some(batch) = self.subscription.next_batch(timeout)? else {
            return Ok(None);
        };
        Ok(Some(self.filter.filter_batch(batch)?))
    }

    pub fn resolve_name(&self, id: &str) -> String {
        self.filter.resolve_name(id)
    }
}

#[derive(Debug, Clone)]
pub struct EventSubtreeFilter {
    scope: EventWatchScope,
    events: Vec<YakEvent>,
}

impl EventSubtreeFilter {
    pub fn new(scope: EventWatchScope, historical_events: Vec<YakEvent>) -> Self {
        Self {
            scope,
            events: historical_events,
        }
    }

    pub fn filter_batch(&mut self, batch: Vec<YakEvent>) -> Result<Vec<YakEvent>> {
        if self.scope == EventWatchScope::All {
            self.events.extend(batch.clone());
            return Ok(batch);
        }

        let mut relevant = Vec::new();
        for event in batch {
            let before = self.current_map()?;
            let relevant_before = self.event_touches_scope(&event, &before);

            self.events.push(event.clone());
            let after = self.current_map()?;
            let relevant_after = self.event_touches_scope(&event, &after);

            if relevant_before || relevant_after {
                relevant.push(event);
            }
        }

        Ok(relevant)
    }

    pub fn resolve_name(&self, id: &str) -> String {
        let id = YakId::from(id);
        self.current_map()
            .ok()
            .and_then(|map| {
                map.snapshot(Vec::new())
                    .yaks
                    .into_iter()
                    .find(|yak| yak.id == id)
                    .map(|yak| yak.name.to_string())
            })
            .unwrap_or_else(|| id.to_string())
    }

    fn current_map(&self) -> Result<YakMap> {
        YakMap::from_events(self.events.clone(), EventMetadata::default_legacy())
    }

    fn event_touches_scope(&self, event: &YakEvent, map: &YakMap) -> bool {
        referenced_yak_ids(event)
            .iter()
            .any(|id| self.id_in_scope(id, map))
    }

    fn id_in_scope(&self, id: &YakId, map: &YakMap) -> bool {
        let EventWatchScope::Subtree(root_id) = &self.scope else {
            return true;
        };
        id == root_id || is_descendant_in_snapshot(map, id, root_id)
    }
}

fn is_descendant_in_snapshot(map: &YakMap, descendant: &YakId, ancestor: &YakId) -> bool {
    let yaks = map.snapshot(Vec::new()).yaks;
    let parents = yaks
        .iter()
        .map(|yak| (yak.id.clone(), yak.parent_id.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    let mut seen = HashSet::new();
    let mut current = parents.get(descendant).and_then(Clone::clone);
    while let Some(parent) = current {
        if &parent == ancestor {
            return true;
        }
        if !seen.insert(parent.clone()) {
            return false;
        }
        current = parents.get(&parent).and_then(Clone::clone);
    }
    false
}

pub fn referenced_yak_ids(event: &YakEvent) -> Vec<YakId> {
    match event {
        YakEvent::Added(e, _) => e
            .parent_id
            .iter()
            .cloned()
            .chain(std::iter::once(e.id.clone()))
            .collect(),
        YakEvent::Removed(e, _) => vec![e.id.clone()],
        YakEvent::Moved(e, _) => e
            .new_parent
            .iter()
            .cloned()
            .chain(std::iter::once(e.id.clone()))
            .collect(),
        YakEvent::FieldUpdated(e, _) => vec![e.id.clone()],
        YakEvent::BlockerAdded(e, _) => blocker_ids(&e.target, &e.blocker.source),
        YakEvent::BlockerUpdated(e, _) => blocker_ids(&e.target, &e.blocker.source),
        YakEvent::BlockerRemoved(e, _) => blocker_ids(&e.target, &e.source),
        YakEvent::ManualBlockerAdded(e, _) => vec![e.target.clone()],
        YakEvent::ManualBlockerUpdated(e, _) => vec![e.target.clone()],
        YakEvent::ManualBlockerRemoved(e, _) => vec![e.target.clone()],
        YakEvent::Compacted(_, _) | YakEvent::Migrated(_, _) => Vec::new(),
    }
}

fn blocker_ids(target: &YakId, source: &BlockerSource) -> Vec<YakId> {
    match source {
        BlockerSource::Yak(blocker) => vec![target.clone(), blocker.clone()],
        BlockerSource::Manual => vec![target.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
    use crate::domain::events::{
        AddedEvent, BlockerRemovedEvent, BlockerSource, FieldUpdatedEvent, MovedEvent, RemovedEvent,
    };
    use crate::domain::Name;

    fn meta() -> EventMetadata {
        EventMetadata::new(
            Author {
                name: "Matt".to_string(),
                email: "matt@example.com".to_string(),
            },
            Timestamp(1),
        )
    }

    fn added(name: &str, id: &str, parent_id: Option<&str>) -> YakEvent {
        YakEvent::Added(
            AddedEvent {
                name: Name::from(name),
                id: YakId::from(id),
                parent_id: parent_id.map(YakId::from),
            },
            meta(),
        )
    }

    fn state(id: &str, content: &str) -> YakEvent {
        YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id: YakId::from(id),
                field_name: ".state".to_string(),
                content: content.to_string(),
            },
            meta(),
        )
    }

    fn moved(id: &str, new_parent: Option<&str>) -> YakEvent {
        YakEvent::Moved(
            MovedEvent {
                id: YakId::from(id),
                new_parent: new_parent.map(YakId::from),
            },
            meta(),
        )
    }

    fn removed(id: &str) -> YakEvent {
        YakEvent::Removed(
            RemovedEvent {
                id: YakId::from(id),
            },
            meta(),
        )
    }

    fn blocker_removed(target: &str, blocker: &str) -> YakEvent {
        YakEvent::BlockerRemoved(
            BlockerRemovedEvent {
                target: YakId::from(target),
                source: BlockerSource::Yak(YakId::from(blocker)),
            },
            meta(),
        )
    }

    fn history() -> Vec<YakEvent> {
        vec![
            added("project", "project-a1b2", None),
            added("fix bug", "fix-bug-c3d4", Some("project-a1b2")),
            added("nested", "nested-e5f6", Some("fix-bug-c3d4")),
            added("admin", "admin-g7h8", None),
        ]
    }

    #[test]
    fn all_scope_returns_every_new_event() {
        let mut filter = EventSubtreeFilter::new(EventWatchScope::All, history());

        let events = filter
            .filter_batch(vec![
                state("admin-g7h8", "done"),
                state("project-a1b2", "done"),
            ])
            .unwrap();

        assert_eq!(events.len(), 2);
    }

    #[test]
    fn subtree_scope_includes_selected_yak_and_descendants() {
        let mut filter = EventSubtreeFilter::new(
            EventWatchScope::Subtree(YakId::from("project-a1b2")),
            history(),
        );

        let events = filter
            .filter_batch(vec![
                state("project-a1b2", "wip"),
                state("fix-bug-c3d4", "done"),
                state("nested-e5f6", "done"),
                state("admin-g7h8", "done"),
            ])
            .unwrap();

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].yak_id(), "project-a1b2");
        assert_eq!(events[1].yak_id(), "fix-bug-c3d4");
        assert_eq!(events[2].yak_id(), "nested-e5f6");
    }

    #[test]
    fn subtree_scope_excludes_ancestors() {
        let mut filter = EventSubtreeFilter::new(
            EventWatchScope::Subtree(YakId::from("fix-bug-c3d4")),
            history(),
        );

        let events = filter
            .filter_batch(vec![state("project-a1b2", "done")])
            .unwrap();

        assert!(events.is_empty());
    }

    #[test]
    fn move_into_subtree_is_relevant_and_later_events_are_included() {
        let mut filter = EventSubtreeFilter::new(
            EventWatchScope::Subtree(YakId::from("project-a1b2")),
            history(),
        );

        let events = filter
            .filter_batch(vec![
                moved("admin-g7h8", Some("project-a1b2")),
                state("admin-g7h8", "done"),
            ])
            .unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].yak_id(), "admin-g7h8");
        assert_eq!(events[1].yak_id(), "admin-g7h8");
    }

    #[test]
    fn move_out_of_subtree_is_relevant_and_later_events_are_excluded() {
        let mut filter = EventSubtreeFilter::new(
            EventWatchScope::Subtree(YakId::from("project-a1b2")),
            history(),
        );

        let events = filter
            .filter_batch(vec![
                moved("fix-bug-c3d4", None),
                state("fix-bug-c3d4", "done"),
            ])
            .unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], YakEvent::Moved(_, _)));
    }

    #[test]
    fn removing_watched_descendant_is_relevant() {
        let mut filter = EventSubtreeFilter::new(
            EventWatchScope::Subtree(YakId::from("project-a1b2")),
            history(),
        );

        let events = filter.filter_batch(vec![removed("nested-e5f6")]).unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], YakEvent::Removed(_, _)));
    }

    #[test]
    fn relationship_events_are_relevant_when_any_referenced_yak_is_in_scope() {
        let mut filter = EventSubtreeFilter::new(
            EventWatchScope::Subtree(YakId::from("project-a1b2")),
            history(),
        );

        let events = filter
            .filter_batch(vec![blocker_removed("admin-g7h8", "fix-bug-c3d4")])
            .unwrap();

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], YakEvent::BlockerRemoved(_, _)));
    }

    struct FakeGlobalEventBus {
        batches: Vec<Vec<YakEvent>>,
    }

    impl GlobalEventBus for FakeGlobalEventBus {
        fn subscribe_from_now(&mut self) -> Result<Box<dyn GlobalEventSubscription + '_>> {
            Ok(Box::new(FakeSubscription {
                batches: &mut self.batches,
            }))
        }
    }

    struct FakeSubscription<'a> {
        batches: &'a mut Vec<Vec<YakEvent>>,
    }

    impl GlobalEventSubscription for FakeSubscription<'_> {
        fn next_batch(&mut self, _timeout: Option<Duration>) -> Result<Option<Vec<YakEvent>>> {
            if self.batches.is_empty() {
                Ok(None)
            } else {
                Ok(Some(self.batches.remove(0)))
            }
        }
    }

    #[test]
    fn resolves_names_from_current_in_memory_projection() {
        let mut filter = EventSubtreeFilter::new(
            EventWatchScope::Subtree(YakId::from("project-a1b2")),
            history(),
        );

        assert_eq!(filter.resolve_name("fix-bug-c3d4"), "fix bug");
        assert_eq!(filter.resolve_name("missing-z9z9"), "missing-z9z9");

        filter
            .filter_batch(vec![moved("admin-g7h8", Some("project-a1b2"))])
            .unwrap();
        assert_eq!(filter.resolve_name("admin-g7h8"), "admin");
    }

    #[test]
    fn watcher_subscribes_from_now_and_does_not_emit_history() {
        let historical_events = history();
        let mut bus = FakeGlobalEventBus {
            batches: vec![vec![state("fix-bug-c3d4", "done")]],
        };
        let mut watcher = EventWatcher::subscribe(
            &mut bus,
            EventWatchScope::Subtree(YakId::from("project-a1b2")),
            historical_events,
        )
        .unwrap();

        let events = watcher.next_relevant_batch(None).unwrap().unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].yak_id(), "fix-bug-c3d4");
    }

    #[test]
    fn watcher_returns_none_when_subscription_times_out() {
        let mut bus = FakeGlobalEventBus {
            batches: Vec::new(),
        };
        let mut watcher = EventWatcher::subscribe(
            &mut bus,
            EventWatchScope::Subtree(YakId::from("project-a1b2")),
            history(),
        )
        .unwrap();

        assert!(watcher
            .next_relevant_batch(Some(Duration::ZERO))
            .unwrap()
            .is_none());
    }
}
