// SyncYaks use case - synchronizes yaks via git refs

use crate::ports::{DisplayPort, SyncPort};
use anyhow::Result;

pub struct SyncYaks<'a> {
    sync: &'a dyn SyncPort,
}

impl<'a> SyncYaks<'a> {
    pub fn new(sync: &'a dyn SyncPort, _display: &'a dyn DisplayPort) -> Self {
        Self { sync }
    }

    pub fn execute(&self) -> Result<()> {
        self.sync.sync()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockSync {
        sync_called: RefCell<bool>,
    }

    impl MockSync {
        fn new() -> Self {
            Self {
                sync_called: RefCell::new(false),
            }
        }

        fn was_sync_called(&self) -> bool {
            *self.sync_called.borrow()
        }
    }

    impl SyncPort for MockSync {
        fn sync(&self) -> Result<()> {
            *self.sync_called.borrow_mut() = true;
            Ok(())
        }
    }

    struct MockOutput {
        messages: RefCell<Vec<String>>,
    }

    impl MockOutput {
        fn new() -> Self {
            Self {
                messages: RefCell::new(Vec::new()),
            }
        }
    }

    impl DisplayPort for MockOutput {
        fn success(&self, message: &str) {
            self.messages.borrow_mut().push(message.to_string());
        }

        fn info(&self, message: &str) {
            self.messages
                .borrow_mut()
                .push(format!("INFO: {}", message));
        }

        fn display_yak_pretty(&self, prefix: &str, name: &str, state: &str) {
            self.messages
                .borrow_mut()
                .push(format!("{prefix}{name} [{state}]"));
        }

        fn display_yak_markdown(&self, depth: usize, name: &str, state: &str) {
            let indent = "  ".repeat(depth);
            self.messages
                .borrow_mut()
                .push(format!("{indent}- [{state}] {name}"));
        }
    }

    #[test]
    fn test_sync_calls_sync_port() {
        let sync = MockSync::new();
        let display = MockOutput::new();
        let use_case = SyncYaks::new(&sync, &display);

        use_case.execute().unwrap();

        assert!(sync.was_sync_called());
    }
}
