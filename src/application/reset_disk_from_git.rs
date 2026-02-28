// Use case: Reset disk projection from git event store
//
// Rebuilds the .yaks directory by replaying all events from the git event store.
// This is the default `yx reset` mode (--disk-from-git).

use anyhow::Result;

use super::{Application, UseCase};

pub struct ResetDiskFromGit;

impl Default for ResetDiskFromGit {
    fn default() -> Self {
        Self
    }
}

impl ResetDiskFromGit {
    pub fn new() -> Self {
        Self
    }
}

impl UseCase for ResetDiskFromGit {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let all_events = app.event_store.get_all_events()?;
        app.event_bus.rebuild(&all_events)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::AddYak;
    use crate::domain::ports::{ReadYakStore, WriteYakStore};
    use crate::domain::slug::YakId;
    use crate::infrastructure::EventBus;

    #[test]
    fn reset_disk_from_git_rebuilds_projection() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            &auth,
        );

        // Add a yak
        app.handle(AddYak::new("test-yak")).unwrap();
        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test-yak")).is_ok());

        // Clear storage to simulate corrupted disk
        storage.clear_all().unwrap();

        // Yak should be gone from storage
        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test-yak")).is_err());

        // Reset disk from git should rebuild it
        app.handle(ResetDiskFromGit::new()).unwrap();

        // Yak should be back
        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test-yak")).is_ok());
    }

    #[test]
    fn reset_disk_from_git_works_with_empty_store() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            &auth,
        );

        // Should succeed even with no events
        app.handle(ResetDiskFromGit::new()).unwrap();
    }
}
