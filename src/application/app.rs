// Application struct - bundles infrastructure adapters for use case execution

use crate::domain::ports::{DisplayPort, EventStoreReader, InputPort, ReadYakStore, SyncPort};
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
}

impl<'a> Application<'a> {
    pub fn new(
        event_bus: &'a mut EventBus,
        store: &'a dyn ReadYakStore,
        display: &'a dyn DisplayPort,
        input: &'a dyn InputPort,
        sync: Option<&'a dyn SyncPort>,
        event_reader: Option<&'a dyn EventStoreReader>,
    ) -> Self {
        Self {
            event_bus,
            store,
            display,
            input,
            sync,
            event_reader,
        }
    }

    pub fn with_yak_map<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut YakMap) -> Result<()>,
    {
        let mut yak_map = YakMap::from_store(self.store)?;
        f(&mut yak_map)?;
        self.save_yak_map(&mut yak_map)?;
        Ok(())
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
    use crate::domain::ports::ReadYakStore;
    use crate::infrastructure::EventBus;

    #[test]
    fn test_application_create_yak_via_yak_map() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        app.with_yak_map(|yak_map| {
            yak_map.add_yak("test".to_string(), None, None)?;
            Ok(())
        })
        .unwrap();

        assert!(ReadYakStore::yak_exists(&storage, "test"));
    }

    #[test]
    fn test_application_mutate_yak_via_yak_map() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        // Create yak and mutate its state via YakMap
        app.with_yak_map(|yak_map| {
            let id = yak_map.add_yak("test".to_string(), None, None)?;
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

        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        // Use YakMap to add a yak
        app.with_yak_map(|yak_map| {
            yak_map.add_yak("test".to_string(), None, Some("context".to_string()))?;
            Ok(())
        })
        .unwrap();

        // Verify yak was created
        assert!(ReadYakStore::yak_exists(&storage, "test"));
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

        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        // Add hierarchical yak
        app.with_yak_map(|yak_map| {
            let parent_id = yak_map.add_yak("parent".to_string(), None, None)?;
            yak_map.add_yak("child".to_string(), Some(parent_id), None)?;
            Ok(())
        })
        .unwrap();

        // Verify both parent and child exist
        assert!(ReadYakStore::yak_exists(&storage, "parent"));
        assert!(ReadYakStore::yak_exists(&storage, "parent/child"));
    }

    #[test]
    fn test_application_with_yak_map_state_propagation() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        // Add hierarchical yak and update child state
        app.with_yak_map(|yak_map| {
            let parent_id = yak_map.add_yak("parent".to_string(), None, None)?;
            let child_id = yak_map.add_yak("child".to_string(), Some(parent_id), None)?;
            yak_map.update_state(child_id, "wip".to_string())
        })
        .unwrap();

        // Verify parent is also wip
        let parent_id = ReadYakStore::fuzzy_find_yak_id(&storage, "parent").unwrap();
        let parent = ReadYakStore::get_yak(&storage, &parent_id).unwrap();
        assert_eq!(parent.state, "wip");
    }
}
