// InProcessWorld - calls CommandHandler directly with in-memory adapters

use anyhow::Result;
use cucumber::World as CucumberWorld;

use super::test_world::TestWorld;
use yx::adapters::{InMemoryDisplay, InMemoryLog, InMemoryStorage};
use yx::cli::CommandHandler;

#[derive(CucumberWorld)]
#[world(init = Self::new)]
pub struct InProcessWorld {
    storage: InMemoryStorage,
    display: InMemoryDisplay,
    log: InMemoryLog,
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
        Ok(Self {
            storage: InMemoryStorage::new(),
            display: InMemoryDisplay::new(),
            log: InMemoryLog::new(),
            exit_code: 0,
        })
    }

    fn handler(&self) -> CommandHandler<'_> {
        CommandHandler::new(&self.storage, &self.display, &self.log)
    }

    fn execute<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&CommandHandler) -> Result<()>,
    {
        self.display.clear();

        let handler = self.handler();
        let result = f(&handler);

        self.exit_code = if result.is_ok() { 0 } else { 1 };

        result
    }
}

impl TestWorld for InProcessWorld {
    fn add_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|h| h.handle_add(name))
    }

    fn list_yaks(&mut self) -> Result<()> {
        self.execute(|h| h.handle_list("pretty", None))
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
