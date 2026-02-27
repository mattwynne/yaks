// Use case: Show yak details (yx show)

use anyhow::Result;

use super::{Application, UseCase};

pub struct ShowYak {
    name: String,
}

impl ShowYak {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;
        let yak = app.store.get_yak(&id)?;

        // Name with state indicator (reuses display_yak_pretty from yx list)
        app.display.display_yak_pretty("", &yak.name, &yak.state);

        // Blank line then metadata
        app.display.info("");
        app.display
            .display_metadata_line(&yak.state, &yak.created_at, &yak.created_by);

        Ok(())
    }
}

impl UseCase for ShowYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::user_display::ConsoleDisplay;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::AddYak;
    use crate::infrastructure::EventBus;

    fn make_app<'a>(
        event_store: &'a mut InMemoryEventStore,
        event_bus: &'a mut EventBus,
        storage: &'a InMemoryStorage,
        display: &'a ConsoleDisplay,
        input: &'a InMemoryInput,
        auth: &'a InMemoryAuthentication,
    ) -> Application<'a> {
        Application::new(event_store, event_bus, storage, display, input, None, auth)
    }

    #[test]
    fn shows_name_with_state_indicator_and_metadata() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        app.handle(AddYak::new("my yak")).unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        // First line: state indicator + name
        assert!(
            lines[0].contains("○ my yak"),
            "Expected state indicator + name, got: {:?}",
            lines[0]
        );

        // Second line: blank
        assert_eq!(lines[1], "", "Expected blank line, got: {:?}", lines[1]);

        // Third line: metadata
        assert!(
            lines[2].starts_with("State: todo"),
            "Expected metadata line, got: {:?}",
            lines[2]
        );
        assert!(
            lines[2].contains("Created:"),
            "Expected created date in metadata, got: {:?}",
            lines[2]
        );
    }

    #[test]
    fn error_when_yak_not_found() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, _buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        let result = app.handle(ShowYak::new("nonexistent"));
        assert!(result.is_err());
    }
}
