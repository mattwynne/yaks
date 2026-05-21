use anyhow::Result;
use git2::{Oid, Repository};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::adapters::event_store::GitEventStore;
use crate::domain::ports::{EventStoreReader, GlobalEventBus, GlobalEventSubscription};
use crate::domain::YakEvent;

pub struct GitGlobalEventBus {
    repo_path: PathBuf,
    ref_name: String,
    poll_interval: Duration,
}

impl GitGlobalEventBus {
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
            ref_name: "refs/notes/yaks".to_string(),
            poll_interval: Duration::from_millis(100),
        }
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    #[cfg(test)]
    fn with_ref_name(mut self, ref_name: &str) -> Self {
        self.ref_name = ref_name.to_string();
        self
    }
}

impl GlobalEventBus for GitGlobalEventBus {
    fn subscribe_from_now(&mut self) -> Result<Box<dyn GlobalEventSubscription + '_>> {
        let repo = Repository::open(&self.repo_path)?;
        let cursor = current_tip(&repo, &self.ref_name)?;
        Ok(Box::new(GitGlobalEventSubscription {
            repo_path: self.repo_path.clone(),
            ref_name: self.ref_name.clone(),
            cursor,
            poll_interval: self.poll_interval,
        }))
    }
}

struct GitGlobalEventSubscription {
    repo_path: PathBuf,
    ref_name: String,
    cursor: Option<Oid>,
    poll_interval: Duration,
}

impl GlobalEventSubscription for GitGlobalEventSubscription {
    fn next_batch(&mut self, timeout: Option<Duration>) -> Result<Option<Vec<YakEvent>>> {
        let started = Instant::now();
        loop {
            if let Some(batch) = self.read_new_batch()? {
                return Ok(Some(batch));
            }

            if let Some(timeout) = timeout {
                let Some(remaining) = timeout.checked_sub(started.elapsed()) else {
                    return Ok(None);
                };
                if remaining.is_zero() {
                    return Ok(None);
                }
                thread::sleep(self.poll_interval.min(remaining));
            } else {
                thread::sleep(self.poll_interval);
            }
        }
    }
}

impl GitGlobalEventSubscription {
    fn read_new_batch(&mut self) -> Result<Option<Vec<YakEvent>>> {
        let repo = Repository::open(&self.repo_path)?;
        let current = current_tip(&repo, &self.ref_name)?;
        if current == self.cursor {
            return Ok(None);
        }

        let events =
            GitEventStore::with_ref_name(&self.repo_path, &self.ref_name)?.get_all_events()?;
        let new_events = events_after_cursor(&repo, &events, self.cursor, current)?;
        self.cursor = current;

        if new_events.is_empty() {
            Ok(None)
        } else {
            Ok(Some(new_events))
        }
    }
}

fn current_tip(repo: &Repository, ref_name: &str) -> Result<Option<Oid>> {
    match repo.refname_to_id(ref_name) {
        Ok(oid) => Ok(Some(oid)),
        Err(err) if err.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn events_after_cursor(
    repo: &Repository,
    events: &[YakEvent],
    cursor: Option<Oid>,
    current: Option<Oid>,
) -> Result<Vec<YakEvent>> {
    let Some(cursor) = cursor else {
        return Ok(events.to_vec());
    };
    let Some(current) = current else {
        anyhow::bail!("event stream was rewritten while watching");
    };

    if !repo.graph_descendant_of(current, cursor)? {
        anyhow::bail!("event stream was rewritten while watching");
    }

    let cursor = cursor.to_string();
    let Some(position) = events
        .iter()
        .position(|event| event.metadata().commit_sha.as_deref() == Some(cursor.as_str()))
    else {
        anyhow::bail!("event stream was compacted or rewritten while watching");
    };

    Ok(events[position + 1..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::event_store::GitEventStore;
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
    use crate::domain::events::AddedEvent;
    use crate::domain::ports::EventStore;
    use crate::domain::{Name, YakId};
    use tempfile::TempDir;

    fn setup() -> (TempDir, GitEventStore, GitGlobalEventBus) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);
        drop(repo);

        let ref_name = "refs/notes/test-yaks-watch";
        let store = GitEventStore::with_ref_name(tmp.path(), ref_name).unwrap();
        let bus = GitGlobalEventBus::new(tmp.path())
            .with_ref_name(ref_name)
            .with_poll_interval(Duration::ZERO);
        (tmp, store, bus)
    }

    fn event(name: &str, id: &str, timestamp: i64) -> YakEvent {
        YakEvent::Added(
            AddedEvent {
                name: Name::from(name),
                id: YakId::from(id),
                parent_id: None,
            },
            EventMetadata::new(
                Author {
                    name: "test".to_string(),
                    email: "test@example.com".to_string(),
                },
                Timestamp(timestamp),
            ),
        )
    }

    #[test]
    fn subscription_from_empty_stream_delivers_first_event() {
        let (_tmp, mut store, mut bus) = setup();
        let mut subscription = bus.subscribe_from_now().unwrap();

        assert!(subscription
            .next_batch(Some(Duration::ZERO))
            .unwrap()
            .is_none());

        store.append(&event("first", "first-a1b2", 1)).unwrap();
        let batch = subscription
            .next_batch(Some(Duration::ZERO))
            .unwrap()
            .unwrap();

        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].yak_id(), "first-a1b2");
    }

    #[test]
    fn subscription_starts_from_current_tip() {
        let (_tmp, mut store, mut bus) = setup();
        store.append(&event("before", "before-a1b2", 1)).unwrap();
        let mut subscription = bus.subscribe_from_now().unwrap();

        assert!(subscription
            .next_batch(Some(Duration::ZERO))
            .unwrap()
            .is_none());

        store.append(&event("after", "after-c3d4", 2)).unwrap();
        let batch = subscription
            .next_batch(Some(Duration::ZERO))
            .unwrap()
            .unwrap();

        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].yak_id(), "after-c3d4");
    }

    #[test]
    fn subscription_returns_multiple_appends_in_order() {
        let (_tmp, mut store, mut bus) = setup();
        store.append(&event("before", "before-a1b2", 1)).unwrap();
        let mut subscription = bus.subscribe_from_now().unwrap();

        store.append(&event("one", "one-c3d4", 2)).unwrap();
        store.append(&event("two", "two-e5f6", 3)).unwrap();
        let batch = subscription
            .next_batch(Some(Duration::ZERO))
            .unwrap()
            .unwrap();

        let ids: Vec<&str> = batch.iter().map(YakEvent::yak_id).collect();
        assert_eq!(ids, vec!["one-c3d4", "two-e5f6"]);
    }

    #[test]
    fn subscription_errors_when_ref_is_rewritten() {
        let (_tmp, mut store, mut bus) = setup();
        store.append(&event("before", "before-a1b2", 1)).unwrap();
        let mut subscription = bus.subscribe_from_now().unwrap();
        store.wipe().unwrap();
        store.append(&event("after", "after-c3d4", 2)).unwrap();

        let err = subscription.next_batch(Some(Duration::ZERO)).unwrap_err();

        assert!(err.to_string().contains("rewritten"));
    }
}
