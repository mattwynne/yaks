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

        // Breadcrumb: walk parent chain to collect ancestor names (root-first)
        let mut ancestors = Vec::new();
        let mut current_parent = yak.parent_id.clone();
        while let Some(pid) = current_parent {
            let parent_yak = app.store.get_yak(&pid)?;
            ancestors.push(parent_yak.name.clone());
            current_parent = parent_yak.parent_id.clone();
        }
        ancestors.reverse();
        app.display.display_breadcrumb(&ancestors);

        // Name with state indicator (reuses display_yak_pretty from yx list)
        app.display.display_yak_pretty("", &yak.name, &yak.state);

        // Blank line then metadata
        app.display.info("");
        app.display
            .display_metadata_line(&yak.state, &yak.created_at, &yak.created_by);

        // Context body (if present and non-empty)
        if let Some(ref context) = yak.context {
            if !context.trim().is_empty() {
                app.display.info("");
                app.display.display_context(context);
            }
        }

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
    use crate::application::{AddYak, EditContext};
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
    fn root_yak_has_no_breadcrumb_line() {
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

        app.handle(AddYak::new("root yak")).unwrap();
        buffer.clear();

        app.handle(ShowYak::new("root yak")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        // First line should be the name, not a breadcrumb
        assert!(
            lines[0].contains("○ root yak"),
            "Expected name as first line, got: {:?}",
            lines[0]
        );
    }

    #[test]
    fn nested_yak_shows_breadcrumb_path() {
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

        app.handle(AddYak::new("grandparent")).unwrap();
        app.handle(AddYak::new("parent").with_parent(Some("grandparent")))
            .unwrap();
        app.handle(AddYak::new("child").with_parent(Some("parent")))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("child")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        // First line: breadcrumb
        assert_eq!(
            lines[0], "grandparent > parent > ",
            "Expected breadcrumb path, got: {:?}",
            lines[0]
        );
        // Second line: name with state indicator
        assert!(
            lines[1].contains("○ child"),
            "Expected name on second line, got: {:?}",
            lines[1]
        );
    }

    #[test]
    fn shows_context_below_metadata() {
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
        input.set_content(Some("Here is some context about this yak.".to_string()));
        app.handle(EditContext::new("my yak")).unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("Here is some context about this yak."),
            "Expected context in output, got: {output}"
        );
        // Context should appear after the metadata line
        let meta_pos = output.find("State:").unwrap();
        let context_pos = output.find("Here is some context").unwrap();
        assert!(
            context_pos > meta_pos,
            "Context should appear after metadata"
        );
    }

    #[test]
    fn no_context_section_when_context_is_empty() {
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
        // Should be just: name, blank, metadata (3 lines)
        assert_eq!(
            lines.len(),
            3,
            "Expected 3 lines (no context section), got {} lines: {:?}",
            lines.len(),
            lines
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
