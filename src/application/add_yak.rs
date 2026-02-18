// Use case: Add a new yak

use crate::domain::validate_yak_name;
use anyhow::Result;

use super::{Application, UseCase};

/// AddYak use case - creates a new yak
pub struct AddYak {
    name: String,
    parent: Option<String>,
}

impl AddYak {
    /// Create a new AddYak use case with the yak name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            parent: None,
        }
    }

    /// Set the parent yak (--under flag)
    pub fn with_parent(mut self, parent: Option<&str>) -> Self {
        self.parent = parent.map(|s| s.to_string());
        self
    }

    /// Execute the use case with the application's infrastructure
    pub fn execute(&self, app: &mut Application) -> Result<()> {
        // Validate user-provided name
        validate_yak_name(&self.name).map_err(|e| anyhow::anyhow!(e))?;

        // Resolve parent to its ID
        let parent_id = if let Some(ref parent_name) = self.parent {
            Some(app.store.fuzzy_find_yak_id(parent_name)?)
        } else {
            None
        };

        // Generate template
        let template = format!("# {}\n\n", self.name);

        // Request content via input port
        let context = app
            .input
            .request_content(None, Some(&template))?
            .filter(|content| !content.trim().is_empty());

        let id = app.with_yak_map_result(|yak_map| {
            yak_map.add_yak(self.name.clone(), parent_id, context, None, None, vec![])
        })?;
        app.display.info(id.as_str());
        Ok(())
    }
}

impl UseCase for AddYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage};
    use crate::domain::ports::ReadYakStore;
    use crate::domain::slug::YakId;
    use crate::infrastructure::EventBus;

    #[test]
    fn test_add_yak_creates_yak() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        let use_case = AddYak::new("test-yak");
        use_case.execute(&mut app).unwrap();

        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test-yak")).is_ok());
    }

    #[test]
    fn test_add_yak_stores_context_from_input() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::with_content("# My context".to_string());
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        AddYak::new("my-yak").execute(&mut app).unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "my-yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.context, Some("# My context".to_string()));
    }

    #[test]
    fn test_add_yak_with_parent() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        AddYak::new("parent").execute(&mut app).unwrap();
        AddYak::new("child")
            .with_parent(Some("parent"))
            .execute(&mut app)
            .unwrap();

        assert!(ReadYakStore::fuzzy_find_yak_id(&storage, "child").is_ok());
    }

    #[test]
    fn test_add_yak_allows_slash_in_name() {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let display = InMemoryDisplay::new();
        let input = InMemoryInput::new();
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        let result = AddYak::new("fix CI/CD pipeline").execute(&mut app);
        assert!(result.is_ok());
    }
}
