// ShowLog use case - displays the event log

use anyhow::Result;
use chrono::DateTime;

use super::{Application, UseCase};

pub struct ShowLog;

impl Default for ShowLog {
    fn default() -> Self {
        Self
    }
}

impl ShowLog {
    pub fn new() -> Self {
        Self
    }
}

impl UseCase for ShowLog {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let reader = app
            .event_reader
            .ok_or_else(|| anyhow::anyhow!("Event reader not configured"))?;
        let events = reader.get_all_events()?;
        for (i, event) in events.iter().enumerate() {
            if i > 0 {
                app.display.info("");
            }
            let meta = event.metadata();
            let datetime = DateTime::from_timestamp(meta.timestamp.as_epoch_secs(), 0)
                .unwrap_or_default();
            let formatted_time = datetime.format("%Y-%m-%d %H:%M").to_string();
            app.display.log_entry(
                &meta.author.name,
                &meta.author.email,
                &formatted_time,
                &event.format_message(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        InMemoryAuthentication, InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage,
    };
    use crate::application::AddYak;
    use crate::infrastructure::EventBus;

    #[test]
    fn test_show_log_displays_events() {
        let event_store = InMemoryEventStore::new();
        let reader = event_store.clone();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let auth = InMemoryAuthentication::new();
        let mut app = Application::new(
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            Some(&reader),
            &auth,
        );

        app.handle(AddYak::new("test yak")).unwrap();
        app.handle(ShowLog::new()).unwrap();
    }

    #[test]
    fn test_show_log_fails_when_not_configured() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let auth = InMemoryAuthentication::new();
        let mut app =
            Application::new(&mut event_bus, &storage, &display, &input, None, None, &auth);

        let result = app.handle(ShowLog::new());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Event reader not configured"
        );
    }
}
