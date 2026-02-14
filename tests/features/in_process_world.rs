// InProcessWorld - calls Application directly with in-memory adapters

use anyhow::Result;
use cucumber::World as CucumberWorld;

use super::test_world::TestWorld;
use yx::adapters::{InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage};
use yx::application::{
    AddYak, Application, DoneYak, EditContext, ListYaks, PruneYaks, RemoveYak, SetState,
    ShowContext, StartYak,
};
use yx::infrastructure::EventBus;

#[derive(CucumberWorld)]
#[world(init = Self::new)]
pub struct InProcessWorld {
    _event_store: InMemoryEventStore,
    event_bus: EventBus,
    storage: InMemoryStorage,
    display: InMemoryDisplay,
    input: InMemoryInput,
    error: String,
    exit_code: i32,
}

impl std::fmt::Debug for InProcessWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessWorld")
            .field("exit_code", &self.exit_code)
            .finish()
    }
}

impl InProcessWorld {
    fn new() -> Result<Self> {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store.clone()));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        Ok(Self {
            _event_store: event_store,
            event_bus,
            storage,
            display: InMemoryDisplay::new(),
            input: InMemoryInput::new(),
            error: String::new(),
            exit_code: 0,
        })
    }

    fn execute<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Application) -> Result<()>,
    {
        self.display.clear();
        self.error.clear();

        let mut app = Application::new(
            &mut self.event_bus,
            &self.storage,
            &self.display,
            &self.input,
            None,
            None,
        );
        let result = f(&mut app);

        self.exit_code = if result.is_ok() { 0 } else { 1 };

        result
    }

    fn try_execute<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Application) -> Result<()>,
    {
        self.display.clear();
        self.error.clear();

        let mut app = Application::new(
            &mut self.event_bus,
            &self.storage,
            &self.display,
            &self.input,
            None,
            None,
        );
        let result = f(&mut app);

        match result {
            Ok(()) => self.exit_code = 0,
            Err(e) => {
                self.exit_code = 1;
                self.error = e.to_string();
            }
        }

        Ok(())
    }
}

impl TestWorld for InProcessWorld {
    fn add_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(AddYak::new(name)))
    }

    fn try_add_yak(&mut self, name: &str) -> Result<()> {
        self.try_execute(|app| app.handle(AddYak::new(name)))
    }

    fn remove_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(RemoveYak::new(name)))
    }

    fn try_remove_yak(&mut self, name: &str) -> Result<()> {
        self.try_execute(|app| app.handle(RemoveYak::new(name)))
    }

    fn get_error(&self) -> String {
        self.error.clone()
    }

    fn done_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(DoneYak::new(name, false)))
    }

    fn list_yaks(&mut self) -> Result<()> {
        self.execute(|app| app.handle(ListYaks::new("pretty", None)))
    }

    fn list_yaks_with_format(&mut self, format: &str) -> Result<()> {
        self.execute(|app| app.handle(ListYaks::new(format, None)))
    }

    fn list_yaks_with_format_and_filter(&mut self, format: &str, only: &str) -> Result<()> {
        self.execute(|app| app.handle(ListYaks::new(format, Some(only))))
    }

    fn set_context(&mut self, name: &str, content: &str) -> Result<()> {
        self.input.set_content(Some(content.to_string()));
        self.execute(|app| app.handle(EditContext::new(name)))
    }

    fn show_context(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(ShowContext::new(name)))
    }

    fn try_done_yak(&mut self, name: &str) -> Result<()> {
        self.try_execute(|app| app.handle(DoneYak::new(name, false)))
    }

    fn done_yak_recursive(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(DoneYak::new(name, true)))
    }

    fn get_output(&self) -> String {
        self.display.get_all_messages().join("\n")
    }

    fn prune_yaks(&mut self) -> Result<()> {
        self.execute(|app| app.handle(PruneYaks::new()))
    }

    fn set_state(&mut self, name: &str, state: &str) -> Result<()> {
        self.execute(|app| app.handle(SetState::new(name, state)))
    }

    fn try_set_state(&mut self, name: &str, state: &str) -> Result<()> {
        self.try_execute(|app| app.handle(SetState::new(name, state)))
    }

    fn start_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(StartYak::new(name, false)))
    }

    fn get_exit_code(&self) -> i32 {
        self.exit_code
    }
}
