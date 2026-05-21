use anyhow::Result;
use std::io::Write;
use std::time::{Duration, Instant};

use super::{Application, EventWatchScope, UseCase};
use crate::application::{event_notification_json_line, yak_event_type, EventWatcher};

pub struct WatchEvents {
    yak: Option<String>,
    timeout: Option<Duration>,
    event_type: Option<String>,
}

impl WatchEvents {
    pub fn new(yak: Option<String>) -> Self {
        Self {
            yak,
            timeout: None,
            event_type: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn waiting_for(mut self, event_type: String) -> Self {
        self.event_type = Some(event_type);
        self
    }
}

impl UseCase for WatchEvents {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let historical_events = app
            .event_reader
            .ok_or_else(|| anyhow::anyhow!("Event reader not configured"))?
            .get_all_events()?;
        let scope = match &self.yak {
            Some(yak) => EventWatchScope::Subtree(app.resolve_yak_id(yak)?),
            None => EventWatchScope::All,
        };
        let global_event_bus = app
            .global_event_bus
            .as_deref_mut()
            .ok_or_else(|| anyhow::anyhow!("Global event bus not configured"))?;
        let mut watcher = EventWatcher::subscribe(global_event_bus, scope, historical_events)?;
        let started = Instant::now();
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        loop {
            let remaining = match self.timeout {
                Some(timeout) => {
                    let Some(remaining) = timeout.checked_sub(started.elapsed()) else {
                        return Ok(());
                    };
                    if remaining.is_zero() {
                        return Ok(());
                    }
                    Some(remaining)
                }
                None => None,
            };

            let Some(events) = watcher.next_relevant_batch(remaining)? else {
                if let Some(event_type) = &self.event_type {
                    anyhow::bail!("timed out waiting for {event_type}");
                }
                return Ok(());
            };
            for event in events {
                if event_matches(&event, self.event_type.as_deref()) {
                    let line =
                        event_notification_json_line(&event, &|id| watcher.resolve_name(id))?;
                    writeln!(stdout, "{line}")?;
                    stdout.flush()?;
                    if self.event_type.is_some() {
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn event_matches(event: &crate::domain::YakEvent, expected_type: Option<&str>) -> bool {
    expected_type
        .map(|expected_type| yak_event_type(event) == expected_type)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::EventMetadata;
    use crate::domain::events::{AddedEvent, RemovedEvent};
    use crate::domain::{Name, YakEvent, YakId};

    #[test]
    fn event_matches_filters_by_event_type_when_present() {
        let added = YakEvent::Added(
            AddedEvent {
                name: Name::from("yak"),
                id: YakId::from("yak-a1b2"),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        );
        let removed = YakEvent::Removed(
            RemovedEvent {
                id: YakId::from("yak-a1b2"),
            },
            EventMetadata::default_legacy(),
        );

        assert!(event_matches(&added, Some("Added")));
        assert!(!event_matches(&removed, Some("Added")));
        assert!(event_matches(&removed, None));
    }
}
