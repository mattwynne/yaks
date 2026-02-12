// InProcessWorld - calls Application directly with in-memory adapters

use anyhow::Result;
use cucumber::World as CucumberWorld;

use super::test_world::TestWorld;
use yx::adapters::{InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage};
use yx::application::{AddYak, Application, ListYaks};
use yx::infrastructure::EventBus;

#[derive(CucumberWorld)]
#[world(init = Self::new)]
pub struct InProcessWorld {
    #[allow(dead_code)]
    event_store: InMemoryEventStore,
    event_bus: EventBus,
    storage: InMemoryStorage,
    display: InMemoryDisplay,
    input: InMemoryInput,
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
            event_store,
            event_bus,
            storage,
            display: InMemoryDisplay::new(),
            input: InMemoryInput::new(),
            exit_code: 0,
        })
    }

    fn execute<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Application) -> Result<()>,
    {
        self.display.clear();

        let mut app = Application::new(
            &mut self.event_bus,
            &self.storage,
            &self.display,
            &self.input,
        );
        let result = f(&mut app);

        self.exit_code = if result.is_ok() { 0 } else { 1 };

        result
    }
}

impl TestWorld for InProcessWorld {
    fn add_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(AddYak::new(name)))
    }

    fn list_yaks(&mut self) -> Result<()> {
        self.execute(|app| app.handle(ListYaks::new("pretty", None)))
    }

    fn get_output(&self) -> String {
        let mut output_lines = Vec::new();
        output_lines.extend(self.display.get_success_messages());
        output_lines.extend(self.display.get_error_messages());
        output_lines.extend(self.display.get_info_messages());
        output_lines.join("\n")
    }

    fn get_exit_code(&self) -> i32 {
        self.exit_code
    }
}
