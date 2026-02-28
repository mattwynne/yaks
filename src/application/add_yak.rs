// Use case: Add a new yak

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::slug::YakId;
use crate::domain::validate_yak_name;
use anyhow::Result;

use super::{Application, UseCase};

/// AddYak use case - creates a new yak
pub struct AddYak {
    name: String,
    parent: Option<String>,
    state: Option<String>,
    context: Option<String>,
    id: Option<String>,
    fields: Vec<(String, String)>,
    author_override: Option<Author>,
    timestamp_override: Option<Timestamp>,
    /// When true, launch $EDITOR for initial context (uses InputPort)
    edit: bool,
    /// When true and no context/edit provided, try reading stdin (uses InputPort)
    read_stdin: bool,
}

impl AddYak {
    /// Create a new AddYak use case with the yak name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            parent: None,
            state: None,
            context: None,
            id: None,
            fields: vec![],
            author_override: None,
            timestamp_override: None,
            edit: false,
            read_stdin: false,
        }
    }

    /// Set the parent yak (--under flag)
    pub fn with_parent(mut self, parent: Option<&str>) -> Self {
        self.parent = parent.map(|s| s.to_string());
        self
    }

    /// Set the initial state (e.g. "wip", "done")
    pub fn with_state(mut self, state: Option<&str>) -> Self {
        self.state = state.map(|s| s.to_string());
        self
    }

    /// Set context directly
    pub fn with_context(mut self, context: Option<&str>) -> Self {
        self.context = context.map(|s| s.to_string());
        self
    }

    /// Set an explicit ID instead of auto-generating one
    pub fn with_id(mut self, id: Option<&str>) -> Self {
        self.id = id.map(|s| s.to_string());
        self
    }

    /// Add a custom field (e.g. "plan", "notes")
    pub fn with_field(mut self, name: &str, value: &str) -> Self {
        self.fields.push((name.to_string(), value.to_string()));
        self
    }

    /// Override the author on the event metadata (used for replaying events)
    pub fn with_author(mut self, author: Option<Author>) -> Self {
        self.author_override = author;
        self
    }

    /// Override the timestamp on the event metadata (used for replaying events)
    pub fn with_timestamp(mut self, timestamp: Option<Timestamp>) -> Self {
        self.timestamp_override = timestamp;
        self
    }

    /// Launch $EDITOR for initial context (handled via InputPort)
    pub fn with_edit(mut self, edit: bool) -> Self {
        self.edit = edit;
        self
    }

    /// Try reading stdin for initial context when no --context or --edit
    pub fn with_read_stdin(mut self, read_stdin: bool) -> Self {
        self.read_stdin = read_stdin;
        self
    }

    /// Execute the use case with the application's infrastructure
    pub fn execute(&self, app: &mut Application) -> Result<()> {
        use crate::domain::event_metadata::EventMetadata;

        // Validate user-provided name
        validate_yak_name(&self.name).map_err(|e| anyhow::anyhow!(e))?;

        // Resolve parent to its ID
        let parent_id = if let Some(ref parent_name) = self.parent {
            Some(app.store.fuzzy_find_yak_id(parent_name)?)
        } else {
            None
        };

        // Resolve context: explicit > editor > stdin
        let context = if self.context.is_some() {
            self.context.clone()
        } else if self.edit {
            let template = format!("# {}\n\n", self.name);
            app.input
                .request_content(None, Some(&template))?
                .filter(|c| !c.trim().is_empty())
        } else if self.read_stdin {
            // Try reading from stdin via input port (returns None if no data)
            app.input.request_content(None, None).ok().flatten()
        } else {
            None
        };

        let metadata = EventMetadata::new(
            self.author_override
                .clone()
                .unwrap_or_else(|| app.current_author()),
            self.timestamp_override.unwrap_or_else(Timestamp::now),
        );

        let id = app.with_yak_map_result_using_metadata(metadata, |yak_map| {
            yak_map.add_yak(
                self.name.clone(),
                parent_id,
                context,
                self.state.clone(),
                self.id.as_ref().map(|s| YakId::from(s.as_str())),
                self.fields.clone(),
            )
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
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::domain::event_metadata::{Author, Timestamp};
    use crate::domain::ports::ReadYakStore;
    use crate::domain::slug::YakId;
    use crate::infrastructure::EventBus;

    #[test]
    fn test_add_yak_creates_yak() {
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

        let use_case = AddYak::new("test-yak");
        use_case.execute(&mut app).unwrap();

        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test-yak")).is_ok());
    }

    #[test]
    fn test_add_yak_without_context_sets_no_context() {
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

        AddYak::new("my-yak").execute(&mut app).unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "my-yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.context, None);
    }

    #[test]
    fn test_add_yak_with_parent() {
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

        AddYak::new("parent").execute(&mut app).unwrap();
        AddYak::new("child")
            .with_parent(Some("parent"))
            .execute(&mut app)
            .unwrap();

        assert!(ReadYakStore::fuzzy_find_yak_id(&storage, "child").is_ok());
    }

    #[test]
    fn test_add_yak_allows_slash_in_name() {
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

        let result = AddYak::new("fix CI/CD pipeline").execute(&mut app);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_yak_with_state() {
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

        AddYak::new("test")
            .with_state(Some("wip"))
            .execute(&mut app)
            .unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "test").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn test_add_yak_with_context_skips_prompt() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        // Set input to return different content - if the prompt is
        // skipped, the yak will have "my notes", not "from input"
        let input = InMemoryInput::with_content("from input".to_string());
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

        AddYak::new("test")
            .with_context(Some("my notes"))
            .execute(&mut app)
            .unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "test").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.context, Some("my notes".to_string()));
    }

    #[test]
    fn test_add_yak_with_explicit_id() {
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

        AddYak::new("test")
            .with_id(Some("custom-id"))
            .execute(&mut app)
            .unwrap();

        assert!(ReadYakStore::get_yak(&storage, &YakId::from("custom-id")).is_ok());
    }

    #[test]
    fn test_add_yak_with_fields() {
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

        AddYak::new("test")
            .with_field("plan", "step 1")
            .execute(&mut app)
            .unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "test").unwrap();
        let content = ReadYakStore::read_field(&storage, &id, "plan").unwrap();
        assert_eq!(content, "step 1");
    }

    #[test]
    fn test_add_yak_with_author_override() {
        use crate::domain::ports::EventStore;

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

        let custom_author = Author {
            name: "Original Author".to_string(),
            email: "original@example.com".to_string(),
        };

        AddYak::new("test")
            .with_author(Some(custom_author.clone()))
            .with_timestamp(Some(Timestamp(1708300800)))
            .execute(&mut app)
            .unwrap();

        let events = EventStore::get_all_events(&event_store).unwrap();
        assert!(!events.is_empty(), "Expected at least one event");
        assert_eq!(events[0].metadata().author, custom_author);
        assert_eq!(events[0].metadata().timestamp, Timestamp(1708300800));
    }

    #[test]
    fn test_add_yak_with_edit_uses_input_port() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::with_content("editor context".to_string());
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

        AddYak::new("test")
            .with_edit(true)
            .execute(&mut app)
            .unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "test").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.context, Some("editor context".to_string()));
    }

    #[test]
    fn test_add_yak_with_read_stdin_uses_input_port() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::with_content("stdin context".to_string());
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

        AddYak::new("test")
            .with_read_stdin(true)
            .execute(&mut app)
            .unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "test").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.context, Some("stdin context".to_string()));
    }
}
