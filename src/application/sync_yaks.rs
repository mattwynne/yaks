// SyncYaks use case - synchronizes yaks via git refs

use anyhow::Result;

use super::{Application, UseCase};

pub struct SyncYaks;

impl SyncYaks {
    pub fn new() -> Self {
        Self
    }
}

impl UseCase for SyncYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let sync = app
            .sync
            .ok_or_else(|| anyhow::anyhow!("Sync not configured"))?;
        sync.sync()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage};
    use crate::infrastructure::EventBus;
    use crate::ports::SyncPort;
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

    #[test]
    fn test_sync_calls_sync_port() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let sync = MockSync::new();

        let mut app = Application::new(
            &mut event_bus,
            &storage,
            &display,
            &input,
            Some(&sync),
        );

        app.handle(SyncYaks::new()).unwrap();

        assert!(sync.was_sync_called());
    }

    #[test]
    fn test_sync_fails_when_not_configured() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
        );

        let result = app.handle(SyncYaks::new());
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Sync not configured"
        );
    }
}
