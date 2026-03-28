// Use case: Show yak context

use crate::adapters::views::Message;
use anyhow::Result;

use super::{Application, UseCase};

pub struct ShowContext {
    name: String,
}

impl ShowContext {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl UseCase for ShowContext {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;
        let yak = app.store.get_yak(&id)?;

        if let Some(context) = &yak.context {
            if !context.is_empty() {
                app.display.message(&Message::Info(context.clone()));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    struct TestWorkspace;

    impl crate::domain::ports::LocalWorkspacePort for TestWorkspace {
        fn is_yaks_gitignored(&self) -> anyhow::Result<bool> {
            Ok(true)
        }

        fn add_yaks_to_gitignore(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn commit_gitignore(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn is_agent_session(&self) -> bool {
            false
        }
    }
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
        workspace: &'a TestWorkspace,
        auth: &'a InMemoryAuthentication,
    ) -> Application<'a> {
        Application::new(
            event_store,
            event_bus,
            storage,
            display,
            input,
            workspace,
            None,
            auth,
        )
    }

    #[test]
    fn shows_context_without_header() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("my yak")).unwrap();
        input.set_content(Some("some context".to_string()));
        app.handle(EditContext::new("my yak")).unwrap();
        buffer.clear();

        app.handle(ShowContext::new("my yak")).unwrap();
        let output = buffer.contents();

        assert_eq!(output, "some context\n");
    }

    #[test]
    fn shows_nothing_when_no_context_is_set() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("my yak")).unwrap();
        buffer.clear();

        app.handle(ShowContext::new("my yak")).unwrap();
        let output = buffer.contents();

        assert_eq!(output, "");
    }
}
