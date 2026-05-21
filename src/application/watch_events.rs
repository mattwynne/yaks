use anyhow::Result;
use std::io::Write;
use std::time::{Duration, Instant};

use super::{Application, EventWatchScope, UseCase};
use crate::application::{event_notification_json_line, EventWatcher};

pub struct WatchEvents {
    yak: Option<String>,
    timeout: Option<Duration>,
}

impl WatchEvents {
    pub fn new(yak: Option<String>) -> Self {
        Self { yak, timeout: None }
    }

    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
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
                return Ok(());
            };
            for event in events {
                let line = event_notification_json_line(&event, &|id| watcher.resolve_name(id))?;
                writeln!(stdout, "{line}")?;
                stdout.flush()?;
            }
        }
    }
}
