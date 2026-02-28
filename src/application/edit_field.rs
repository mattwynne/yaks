// Use case: Edit a yak field interactively

use crate::domain::validate_field_name;
use anyhow::Result;

use super::{Application, UseCase};

pub struct EditField {
    name: String,
    field: String,
    /// Optional override for initial content (e.g. from piped stdin)
    initial_content: Option<String>,
}

impl EditField {
    pub fn new(name: &str, field: &str) -> Self {
        Self {
            name: name.to_string(),
            field: field.to_string(),
            initial_content: None,
        }
    }

    /// Override the initial content shown in the editor.
    /// When not set, the existing field value is used.
    pub fn with_initial_content(mut self, content: &str) -> Self {
        self.initial_content = Some(content.to_string());
        self
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        validate_field_name(&self.field)?;

        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        // Use provided initial content, or read existing field value
        let initial = if let Some(ref content) = self.initial_content {
            content.clone()
        } else {
            let yak = app.store.get_yak(&id)?;
            yak.fields.get(&self.field).cloned().unwrap_or_default()
        };

        // Request new content via input port (editor or test fixture)
        if let Some(content) = app.input.request_content(Some(&initial), None)? {
            let field = self.field.clone();
            app.with_yak_map(|yak_map| yak_map.update_field(id.clone(), field, content))?;
        }

        Ok(())
    }
}

impl UseCase for EditField {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{make_test_display, InMemoryEventStore, InMemoryInput, InMemoryStorage};
    use crate::application::AddYak;
    use crate::domain::event_metadata::Author;
    use crate::domain::ports::{AuthenticationPort, ReadYakStore};
    use crate::infrastructure::EventBus;

    struct TestAuth;
    impl AuthenticationPort for TestAuth {
        fn current_author(&self) -> Author {
            Author {
                name: "test".to_string(),
                email: "test@test.com".to_string(),
            }
        }
    }

    fn setup() -> (
        InMemoryEventStore,
        EventBus,
        InMemoryStorage,
        crate::adapters::user_display::ConsoleDisplay,
        crate::adapters::TestBuffer,
        InMemoryInput,
        TestAuth,
    ) {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = TestAuth;
        (
            event_store,
            event_bus,
            storage,
            display,
            buffer,
            input,
            auth,
        )
    }

    #[test]
    fn edit_field_saves_content_from_input_port() {
        let (mut event_store, mut event_bus, storage, display, _buffer, input, auth) = setup();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            app.handle(AddYak::new("my yak").with_field("notes", "old value"))
                .unwrap();
        }

        input.set_content(Some("new value".to_string()));

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            app.handle(EditField::new("my yak", "notes")).unwrap();
        }

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "my yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.fields.get("notes"), Some(&"new value".to_string()));
    }

    #[test]
    fn edit_field_no_op_when_input_returns_none() {
        let (mut event_store, mut event_bus, storage, display, _buffer, input, auth) = setup();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            app.handle(AddYak::new("my yak").with_field("notes", "original"))
                .unwrap();
        }

        input.set_content(None);

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            app.handle(EditField::new("my yak", "notes")).unwrap();
        }

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "my yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.fields.get("notes"), Some(&"original".to_string()));
    }

    #[test]
    fn edit_field_works_when_field_does_not_exist_yet() {
        let (mut event_store, mut event_bus, storage, display, _buffer, input, auth) = setup();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            app.handle(AddYak::new("my yak")).unwrap();
        }

        input.set_content(Some("brand new".to_string()));

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            app.handle(EditField::new("my yak", "notes")).unwrap();
        }

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "my yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.fields.get("notes"), Some(&"brand new".to_string()));
    }

    #[test]
    fn edit_field_validates_field_name() {
        let (mut event_store, mut event_bus, storage, display, _buffer, input, auth) = setup();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            app.handle(AddYak::new("my yak")).unwrap();
        }

        input.set_content(Some("value".to_string()));

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            let result = app.handle(EditField::new("my yak", ".state"));
            assert!(result.is_err());
        }
    }

    #[test]
    fn edit_field_with_initial_content_overrides_existing_field() {
        let (mut event_store, mut event_bus, storage, display, _buffer, input, auth) = setup();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            app.handle(AddYak::new("my yak").with_field("notes", "old value"))
                .unwrap();
        }

        // Input returns "edited content" (simulating editor output)
        input.set_content(Some("edited content".to_string()));

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );
            // Use with_initial_content to override what the editor sees
            app.handle(EditField::new("my yak", "notes").with_initial_content("piped content"))
                .unwrap();
        }

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "my yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.fields.get("notes"), Some(&"edited content".to_string()));
    }
}
