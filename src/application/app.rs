// Application struct - bundles infrastructure adapters for use case execution

#[cfg(test)]
use crate::domain::validate_yak_name;
use crate::domain::{Yak, YakMap};
use crate::infrastructure::EventBus;
use crate::ports::{DisplayPort, InputPort, Store};
use anyhow::Result;

use super::UseCase;

/// Application bundles the infrastructure adapters needed by use cases
///
/// This struct represents the application layer's view of infrastructure.
/// Use cases are constructed with domain data, then executed with an Application.
pub struct Application<'a> {
    event_bus: &'a mut EventBus,
    pub store: &'a dyn Store,
    pub display: &'a dyn DisplayPort,
    pub input: &'a dyn InputPort,
}

impl<'a> Application<'a> {
    pub fn new(
        event_bus: &'a mut EventBus,
        store: &'a dyn Store,
        display: &'a dyn DisplayPort,
        input: &'a dyn InputPort,
    ) -> Self {
        Self {
            event_bus,
            store,
            display,
            input,
        }
    }

    pub fn with_yak<F>(&mut self, name: &str, f: F) -> Result<()>
    where
        F: FnOnce(&mut Yak) -> Result<()>,
    {
        let yak_name = self.store.find_yak(name)?;
        let mut yak = self.store.get_yak(&yak_name)?;
        f(&mut yak)?;
        self.save(&mut yak)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn with_new_yak<F>(&mut self, name: &str, f: F) -> Result<()>
    where
        F: FnOnce(&mut Yak) -> Result<()>,
    {
        validate_yak_name(name).map_err(|e| anyhow::anyhow!(e))?;
        let mut yak = Yak::new(name.to_string());
        f(&mut yak)?;
        self.save(&mut yak)?;
        Ok(())
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

    fn save(&mut self, aggregate: &mut Yak) -> Result<()> {
        for event in aggregate.take_events() {
            self.event_bus.publish(event)?;
        }
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
    /// let app = Application::new(&mut event_bus, &store, &display, &input);
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
    use crate::infrastructure::EventBus;
    use crate::ports::Store;

    #[test]
    fn test_application_with_new_yak() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(&mut event_bus, &storage, &display, &input);

        app.with_new_yak("test", |yak| {
            assert_eq!(yak.name, "test");
            Ok(())
        })
        .unwrap();

        assert!(Store::yak_exists(&storage, "test"));
    }

    #[test]
    fn test_application_with_yak() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(&mut event_bus, &storage, &display, &input);

        // Create yak first
        app.with_new_yak("test", |_| Ok(())).unwrap();

        // Now mutate it
        app.with_yak("test", |yak| yak.update_state("wip".to_string()))
            .unwrap();

        let yak = Store::get_yak(&storage, "test").unwrap();
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

        let mut app = Application::new(&mut event_bus, &storage, &display, &input);

        // Use YakMap to add a yak
        app.with_yak_map(|yak_map| {
            yak_map.add_yak("test".to_string(), Some("context".to_string()))
        })
        .unwrap();

        // Verify yak was created
        assert!(Store::yak_exists(&storage, "test"));
        let yak = Store::get_yak(&storage, "test").unwrap();
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

        let mut app = Application::new(&mut event_bus, &storage, &display, &input);

        // Add hierarchical yak
        app.with_yak_map(|yak_map| yak_map.add_yak("parent/child".to_string(), None))
            .unwrap();

        // Verify both parent and child exist
        assert!(Store::yak_exists(&storage, "parent"));
        assert!(Store::yak_exists(&storage, "parent/child"));
    }

    #[test]
    fn test_application_with_yak_map_state_propagation() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();

        let mut app = Application::new(&mut event_bus, &storage, &display, &input);

        // Add hierarchical yak and update child state
        app.with_yak_map(|yak_map| {
            yak_map.add_yak("parent/child".to_string(), None)?;
            yak_map.update_state("parent/child".to_string(), "wip".to_string())
        })
        .unwrap();

        // Verify parent is also wip
        let parent = Store::get_yak(&storage, "parent").unwrap();
        assert_eq!(parent.state, "wip");
    }
}
