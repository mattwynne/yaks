// Application struct - bundles infrastructure adapters for use case execution

use crate::domain::ports::{
    AuthenticationPort, DisplayPort, EventStoreReader, InputPort, ReadYakStore, SyncPort,
};
use crate::domain::YakMap;
use crate::infrastructure::EventBus;
use anyhow::Result;

use super::UseCase;

/// Application bundles the infrastructure adapters needed by use cases
///
/// This struct represents the application layer's view of infrastructure.
/// Use cases are constructed with domain data, then executed with an Application.
pub struct Application<'a> {
    event_bus: &'a mut EventBus,
    pub store: &'a dyn ReadYakStore,
    pub display: &'a dyn DisplayPort,
    pub input: &'a dyn InputPort,
    pub sync: Option<&'a dyn SyncPort>,
    pub event_reader: Option<&'a dyn EventStoreReader>,
    auth: &'a dyn AuthenticationPort,
}

impl<'a> Application<'a> {
    pub fn new(
        event_bus: &'a mut EventBus,
        store: &'a dyn ReadYakStore,
        display: &'a dyn DisplayPort,
        input: &'a dyn InputPort,
        sync: Option<&'a dyn SyncPort>,
        event_reader: Option<&'a dyn EventStoreReader>,
        auth: &'a dyn AuthenticationPort,
    ) -> Self {
        Self {
            event_bus,
            store,
            display,
            input,
            sync,
            event_reader,
            auth,
        }
    }

    pub fn with_yak_map<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut YakMap) -> Result<()>,
    {
        use crate::domain::event_metadata::{EventMetadata, Timestamp};
        let metadata = EventMetadata::new(self.auth.current_author(), Timestamp::now());
        let mut yak_map = YakMap::from_store(self.store, metadata)?;
        f(&mut yak_map)?;
        self.save_yak_map(&mut yak_map)?;
        Ok(())
    }

    pub fn with_yak_map_result<T, F>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce(&mut YakMap) -> Result<T>,
    {
        use crate::domain::event_metadata::{EventMetadata, Timestamp};
        let metadata = EventMetadata::new(self.auth.current_author(), Timestamp::now());
        let mut yak_map = YakMap::from_store(self.store, metadata)?;
        let result = f(&mut yak_map)?;
        self.save_yak_map(&mut yak_map)?;
        Ok(result)
    }

    fn save_yak_map(&mut self, yak_map: &mut YakMap) -> Result<()> {
        for event in yak_map.take_events() {
            self.event_bus.publish(event)?;
        }
        Ok(())
    }

    /// Execute a use case with this application's infrastructure
    ///
    /// # Example
    /// ```ignore
    /// let app = Application::new(&mut event_bus, &store, &display, &input, None, None);
    /// app.handle(AddYak::new("my yak"))?;
    /// ```
    pub fn handle<U: UseCase>(&mut self, use_case: U) -> Result<()> {
        use_case.execute(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage};
    use crate::domain::event_metadata::Author;
    use crate::domain::ports::{AuthenticationPort, ReadYakStore};
    use crate::domain::slug::YakId;
    use crate::infrastructure::EventBus;

    struct TestAuth {
        name: String,
        email: String,
    }

    impl TestAuth {
        fn new(name: &str, email: &str) -> Self {
            Self {
                name: name.to_string(),
                email: email.to_string(),
            }
        }
    }

    impl AuthenticationPort for TestAuth {
        fn current_author(&self) -> Author {
            Author {
                name: self.name.clone(),
                email: self.email.clone(),
            }
        }
    }

    #[test]
    fn test_application_stamps_author_on_events() {
        use crate::domain::ports::EventStore;

        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store.clone()));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("Test Author", "test@example.com");

        let mut app = Application::new(
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            None,
            &auth,
        );

        app.with_yak_map(|yak_map| {
            yak_map.add_yak("test".to_string(), None, None, None, None, vec![])?;
            Ok(())
        })
        .unwrap();

        let events = EventStore::get_all_events(&event_store).unwrap();
        assert!(!events.is_empty(), "Expected at least one event");
        let first_event = &events[0];
        let metadata = first_event.metadata();
        assert_eq!(
            metadata.author.name,
            "Test Author",
            "Event should carry author from auth port"
        );
        assert_eq!(
            metadata.author.email,
            "test@example.com",
            "Event should carry email from auth port"
        );
    }

    #[test]
    fn test_application_create_yak_via_yak_map() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");

        let mut app =
            Application::new(&mut event_bus, &storage, &display, &input, None, None, &auth);

        app.with_yak_map(|yak_map| {
            yak_map.add_yak("test".to_string(), None, None, None, None, vec![])?;
            Ok(())
        })
        .unwrap();

        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test")).is_ok());
    }

    #[test]
    fn test_application_mutate_yak_via_yak_map() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");

        let mut app =
            Application::new(&mut event_bus, &storage, &display, &input, None, None, &auth);

        // Create yak and mutate its state via YakMap
        app.with_yak_map(|yak_map| {
            let id = yak_map.add_yak("test".to_string(), None, None, None, None, vec![])?;
            yak_map.update_state(id, "wip".to_string())
        })
        .unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "test").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn test_application_with_yak_map() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");

        let mut app =
            Application::new(&mut event_bus, &storage, &display, &input, None, None, &auth);

        // Use YakMap to add a yak
        app.with_yak_map(|yak_map| {
            yak_map.add_yak(
                "test".to_string(),
                None,
                Some("context".to_string()),
                None,
                None,
                vec![],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify yak was created
        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test")).is_ok());
        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "test").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, "todo");
        assert_eq!(yak.context, Some("context".to_string()));
    }

    #[test]
    fn test_application_with_yak_map_hierarchy() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");

        let mut app =
            Application::new(&mut event_bus, &storage, &display, &input, None, None, &auth);

        // Add hierarchical yak
        app.with_yak_map(|yak_map| {
            let parent_id =
                yak_map.add_yak("parent".to_string(), None, None, None, None, vec![])?;
            yak_map.add_yak(
                "child".to_string(),
                Some(parent_id),
                None,
                None,
                None,
                vec![],
            )?;
            Ok(())
        })
        .unwrap();

        // Verify both parent and child exist
        assert!(ReadYakStore::get_yak(&storage, &YakId::from("parent")).is_ok());
        assert!(ReadYakStore::fuzzy_find_yak_id(&storage, "child").is_ok());
    }

    #[test]
    fn test_application_with_yak_map_state_propagation() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");

        let mut app =
            Application::new(&mut event_bus, &storage, &display, &input, None, None, &auth);

        // Add hierarchical yak and update child state
        app.with_yak_map(|yak_map| {
            let parent_id =
                yak_map.add_yak("parent".to_string(), None, None, None, None, vec![])?;
            let child_id = yak_map.add_yak(
                "child".to_string(),
                Some(parent_id),
                None,
                None,
                None,
                vec![],
            )?;
            yak_map.update_state(child_id, "wip".to_string())
        })
        .unwrap();

        // Verify parent is also wip
        let parent_id = ReadYakStore::fuzzy_find_yak_id(&storage, "parent").unwrap();
        let parent = ReadYakStore::get_yak(&storage, &parent_id).unwrap();
        assert_eq!(parent.state, "wip");
    }
}
