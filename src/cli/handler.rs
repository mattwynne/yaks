// Command handler - coordinates ports and use cases for CLI commands

use crate::application::{
    AddYak, DoneYak, EditContext, ListYaks, MoveYak, PruneYaks, RemoveYak, SetState, ShowContext,
    ShowField, WriteField,
};
use crate::ports::{LogPort, OutputPort, StoragePort};
use anyhow::Result;

/// CommandHandler coordinates CLI commands with injected ports
///
/// This struct allows commands to be tested with in-memory adapters
/// while the real binary uses filesystem/git adapters.
pub struct CommandHandler<'a> {
    storage: &'a dyn StoragePort,
    output: &'a dyn OutputPort,
    log: &'a dyn LogPort,
}

impl<'a> CommandHandler<'a> {
    /// Create a new CommandHandler with injected dependencies
    pub fn new(
        storage: &'a dyn StoragePort,
        output: &'a dyn OutputPort,
        log: &'a dyn LogPort,
    ) -> Self {
        Self {
            storage,
            output,
            log,
        }
    }

    /// Handle the 'add' command
    pub fn handle_add(&self, name: &str) -> Result<()> {
        let use_case = AddYak::new(self.storage, self.output, self.log);
        use_case.execute(name)
    }

    /// Handle the 'list' command
    pub fn handle_list(&self, format: &str, only: Option<&str>) -> Result<()> {
        let use_case = ListYaks::new(self.storage, self.output);
        use_case.execute(format, only)
    }

    /// Handle the 'done' command
    pub fn handle_done(&self, name: &str, undo: bool, recursive: bool) -> Result<()> {
        let use_case = DoneYak::new(self.storage, self.output, self.log);
        use_case.execute(name, undo, recursive)
    }

    /// Handle the 'remove' command
    pub fn handle_remove(&self, name: &str) -> Result<()> {
        let use_case = RemoveYak::new(self.storage, self.output, self.log);
        use_case.execute(name)
    }

    /// Handle the 'prune' command
    pub fn handle_prune(&self) -> Result<()> {
        let use_case = PruneYaks::new(self.storage, self.output, self.log);
        use_case.execute()
    }

    /// Handle the 'move' command
    pub fn handle_move(&self, from: &str, to: &str) -> Result<()> {
        let use_case = MoveYak::new(self.storage, self.output, self.log);
        use_case.execute(from, to)
    }

    /// Handle the 'context' command (show variant)
    pub fn handle_context_show(&self, name: &str) -> Result<()> {
        let use_case = ShowContext::new(self.storage, self.output);
        use_case.execute(name)
    }

    /// Handle the 'context' command (edit variant)
    pub fn handle_context_edit(&self, name: &str) -> Result<()> {
        let use_case = EditContext::new(self.storage, self.output, self.log);
        use_case.execute(name)
    }

    /// Handle the 'state' command
    pub fn handle_state(&self, name: &str, state: &str) -> Result<()> {
        let use_case = SetState::new(self.storage, self.output, self.log);
        use_case.execute(name, state)
    }

    /// Handle the 'field' command (show variant)
    pub fn handle_field_show(&self, name: &str, field: &str) -> Result<()> {
        let use_case = ShowField::new(self.storage, self.output, self.log);
        use_case.execute(name, field)
    }

    /// Handle the 'field' command (write variant)
    pub fn handle_field_write(&self, name: &str, field: &str) -> Result<()> {
        let use_case = WriteField::new(self.storage, self.output, self.log);
        use_case.execute(name, field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{InMemoryLog, InMemoryOutput, InMemoryStorage};

    #[test]
    fn test_add_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_add("test yak");

        assert!(result.is_ok());
        let yaks = storage.list_yaks().unwrap();
        assert_eq!(yaks.len(), 1);
        assert_eq!(yaks[0].name, "test yak");
    }

    #[test]
    fn test_list_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        // Add yaks first
        storage.create_yak("yak one").unwrap();
        storage.create_yak("yak two").unwrap();

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_list("plain", None);

        assert!(result.is_ok());
        let messages = output.get_info_messages();
        assert!(messages.iter().any(|m| m.contains("yak one")));
        assert!(messages.iter().any(|m| m.contains("yak two")));
    }

    #[test]
    fn test_done_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        // Add a yak first
        storage.create_yak("test yak").unwrap();

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_done("test yak", false, false);

        assert!(result.is_ok());
        let yak = storage.get_yak("test yak").unwrap();
        assert!(yak.done);
    }

    #[test]
    fn test_remove_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        // Add a yak first
        storage.create_yak("test yak").unwrap();
        assert!(storage.get_yak("test yak").is_ok());

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_remove("test yak");

        assert!(result.is_ok());
        assert!(storage.get_yak("test yak").is_err());
    }

    #[test]
    fn test_prune_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        // Add yaks
        storage.create_yak("not done").unwrap();
        storage.create_yak("done yak").unwrap();

        // Mark one as done
        storage.write_field("done yak", "state", "done").unwrap();

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_prune();

        assert!(result.is_ok());
        let yaks = storage.list_yaks().unwrap();
        assert_eq!(yaks.len(), 1);
        assert_eq!(yaks[0].name, "not done");
    }

    #[test]
    fn test_move_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        // Add a yak
        storage.create_yak("old name").unwrap();

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_move("old name", "new name");

        assert!(result.is_ok());
        assert!(storage.get_yak("old name").is_err());
        assert!(storage.get_yak("new name").is_ok());
    }

    #[test]
    fn test_state_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        // Add a yak
        storage.create_yak("test yak").unwrap();

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_state("test yak", "wip");

        assert!(result.is_ok());
        let yak = storage.get_yak("test yak").unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn test_context_show_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        // Add a yak with context
        storage.create_yak("test yak").unwrap();
        storage
            .write_field("test yak", "context.md", "Test context")
            .unwrap();

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_context_show("test yak");

        assert!(result.is_ok());
        let messages = output.get_info_messages();
        assert!(messages.iter().any(|m| m.contains("Test context")));
    }

    #[test]
    fn test_field_show_with_in_memory_adapters() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        // Add a yak with field
        storage.create_yak("test yak").unwrap();
        storage
            .write_field("test yak", "notes", "Test notes")
            .unwrap();

        let handler = CommandHandler::new(&storage, &output, &log);
        let result = handler.handle_field_show("test yak", "notes");

        assert!(result.is_ok());
        let messages = output.get_info_messages();
        assert!(messages.iter().any(|m| m.contains("Test notes")));
    }

    #[test]
    fn test_multiple_commands_share_state() {
        let storage = InMemoryStorage::new();
        let output = InMemoryOutput::new();
        let log = InMemoryLog::new();

        let handler = CommandHandler::new(&storage, &output, &log);

        // Add a yak
        handler.handle_add("yak one").unwrap();

        // List should show it
        output.clear();
        handler.handle_list("plain", None).unwrap();
        let messages = output.get_info_messages();
        assert!(messages.iter().any(|m| m.contains("yak one")));

        // Mark it done
        handler.handle_done("yak one", false, false).unwrap();

        // Verify it's done
        let yak = storage.get_yak("yak one").unwrap();
        assert!(yak.done);
    }
}
