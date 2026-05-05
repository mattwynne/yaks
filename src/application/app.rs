// Application struct - bundles infrastructure adapters for use case execution

use crate::domain::ports::{
    AuthenticationPort, DisplayPort, EventStore, EventStoreReader, InputPort, LocalWorkspacePort,
    ReadYakStore, UserConfigPort,
};
use crate::domain::slug::YakId;
use crate::domain::YakMap;
use crate::infrastructure::EventBus;
use anyhow::Result;
#[cfg(any(test, feature = "test-support"))]
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[cfg(any(test, feature = "test-support"))]
thread_local! {
    static FOCUS_OVERRIDE: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg(any(test, feature = "test-support"))]
pub fn set_focus_override(focus: Option<&str>) {
    FOCUS_OVERRIDE.with(|cell| *cell.borrow_mut() = focus.map(|s| s.to_string()));
}

use super::{CommandHandler, UseCase};

/// Application bundles the infrastructure adapters needed by use cases
///
/// This struct represents the application layer's view of infrastructure.
/// Use cases are constructed with domain data, then executed with an Application.
pub struct Application<'a> {
    pub(super) event_store: &'a mut dyn EventStore,
    pub(super) event_bus: &'a mut EventBus,
    pub store: &'a dyn ReadYakStore,
    pub display: &'a dyn DisplayPort,
    pub input: &'a dyn InputPort,
    pub local_workspace: &'a dyn LocalWorkspacePort,
    pub event_reader: Option<&'a dyn EventStoreReader>,
    auth: &'a dyn AuthenticationPort,
    pub(super) user_config: Option<&'a mut dyn UserConfigPort>,
}

impl<'a> Application<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_store: &'a mut dyn EventStore,
        event_bus: &'a mut EventBus,
        store: &'a dyn ReadYakStore,
        display: &'a dyn DisplayPort,
        input: &'a dyn InputPort,
        local_workspace: &'a dyn LocalWorkspacePort,
        event_reader: Option<&'a dyn EventStoreReader>,
        auth: &'a dyn AuthenticationPort,
    ) -> Self {
        Self {
            event_store,
            event_bus,
            store,
            display,
            input,
            local_workspace,
            event_reader,
            auth,
            user_config: None,
        }
    }

    /// Attach a user config port (builder pattern).
    pub fn with_user_config(mut self, config: &'a mut dyn UserConfigPort) -> Self {
        self.user_config = Some(config);
        self
    }

    pub fn focus_id(&self) -> Result<Option<YakId>> {
        #[cfg(any(test, feature = "test-support"))]
        let override_focus = FOCUS_OVERRIDE.with(|cell| cell.borrow().clone());
        #[cfg(not(any(test, feature = "test-support")))]
        let override_focus: Option<String> = None;

        let focus = if let Some(focus) = override_focus {
            focus
        } else {
            let Some(raw) = std::env::var_os("YX_FOCUS") else {
                return Ok(None);
            };
            raw.to_string_lossy().to_string()
        };
        let yaks = self.store.list_yaks()?;
        if yaks.iter().any(|y| y.id.as_str() == focus) {
            Ok(Some(YakId::from(focus.as_str())))
        } else {
            anyhow::bail!("YX_FOCUS '{}' does not exactly match a yak id", focus)
        }
    }

    pub fn focused_yak_ids(&self) -> Result<Option<HashSet<YakId>>> {
        let Some(focus_id) = self.focus_id()? else {
            return Ok(None);
        };
        let yaks = self.store.list_yaks()?;
        Ok(Some(focused_ids_from_yaks(&yaks, &focus_id)))
    }

    pub fn is_yak_visible(&self, id: &YakId) -> Result<bool> {
        Ok(self
            .focused_yak_ids()?
            .map(|ids| ids.contains(id))
            .unwrap_or(true))
    }

    pub fn ensure_yak_visible(&self, id: &YakId) -> Result<()> {
        if self.is_yak_visible(id)? {
            Ok(())
        } else {
            anyhow::bail!("yak '{}' is outside YX_FOCUS", id.as_str())
        }
    }

    pub fn resolve_yak_id(&self, query: &str) -> Result<YakId> {
        let id = self.store.fuzzy_find_yak_id(query)?;
        self.ensure_yak_visible(&id)?;
        Ok(id)
    }

    pub fn with_yak_map<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut YakMap) -> Result<()>,
    {
        use crate::domain::event_metadata::{EventMetadata, Timestamp};
        let metadata = EventMetadata::new(self.auth.current_author(), Timestamp::now());
        self.with_yak_map_result_using_metadata(metadata, f)
    }

    pub fn with_yak_map_result<T, F>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce(&mut YakMap) -> Result<T>,
    {
        use crate::domain::event_metadata::{EventMetadata, Timestamp};
        let metadata = EventMetadata::new(self.auth.current_author(), Timestamp::now());
        self.with_yak_map_result_using_metadata(metadata, f)
    }

    pub fn with_yak_map_result_using_metadata<T, F>(
        &mut self,
        metadata: crate::domain::event_metadata::EventMetadata,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce(&mut YakMap) -> Result<T>,
    {
        let mut yak_map = if let Some(reader) = self.event_reader {
            // Event-sourced: replay events from event store
            let events = reader.get_all_events()?;
            YakMap::from_events(events, metadata)?
        } else {
            // Fallback: load from read model (for backward compatibility)
            YakMap::from_store(self.store, metadata)?
        };
        let result = f(&mut yak_map)?;
        self.save_yak_map(&mut yak_map)?;
        Ok(result)
    }

    /// Returns the current author from the authentication port
    pub fn current_author(&self) -> crate::domain::event_metadata::Author {
        self.auth.current_author()
    }

    /// Sync events with a remote peer
    ///
    /// Delegates to the event store's sync method, then rebuilds
    /// the disk projection from the full event history. The
    /// rebuild handles worktrees (which share a git repo and
    /// therefore already have local events that haven't been
    /// projected to their .yaks dir).
    pub fn sync_events(&mut self) -> Result<()> {
        self.event_store.sync(self.event_bus, self.display)?;

        // Rebuild projection: clear storage and replay all events.
        // This ensures the disk is consistent even when the
        // local event store already had events (e.g. worktrees
        // sharing a git repo).
        let all_events = self.event_store.get_all_events()?;
        self.event_bus.rebuild(&all_events)?;

        Ok(())
    }

    fn save_yak_map(&mut self, yak_map: &mut YakMap) -> Result<()> {
        for event in yak_map.take_events() {
            self.event_store.append(&event)?;
            self.event_bus.notify(&event)?;
        }
        Ok(())
    }

    /// Execute a use case with this application's infrastructure
    ///
    /// # Example
    /// ```ignore
    /// let app = Application::new(&mut event_store, &mut event_bus, &store, &display, &input, None, &auth);
    /// app.handle(AddYak::new("my yak"))?;
    /// ```
    pub fn handle<U: UseCase>(&mut self, use_case: U) -> Result<()> {
        use_case.execute(self)
    }
}

fn focused_ids_from_yaks(yaks: &[crate::domain::Yak], focus_id: &YakId) -> HashSet<YakId> {
    let by_id: HashMap<YakId, &crate::domain::Yak> =
        yaks.iter().map(|y| (y.id.clone(), y)).collect();
    let mut ids = HashSet::new();

    let mut current = Some(focus_id.clone());
    while let Some(id) = current {
        if !ids.insert(id.clone()) {
            break;
        }
        current = by_id.get(&id).and_then(|y| y.parent_id.clone());
    }

    let mut children_by_parent: HashMap<YakId, Vec<YakId>> = HashMap::new();
    for yak in yaks {
        if let Some(parent_id) = &yak.parent_id {
            children_by_parent
                .entry(parent_id.clone())
                .or_default()
                .push(yak.id.clone());
        }
    }
    let mut stack = children_by_parent
        .get(focus_id)
        .cloned()
        .unwrap_or_default();
    while let Some(id) = stack.pop() {
        if ids.insert(id.clone()) {
            if let Some(children) = children_by_parent.get(&id) {
                stack.extend(children.iter().cloned());
            }
        }
    }

    ids
}

impl<'a> CommandHandler for Application<'a> {
    fn handle(&mut self, use_case: impl UseCase) -> Result<()> {
        use_case.execute(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{make_test_display, InMemoryEventStore, InMemoryInput, InMemoryStorage};
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

    struct TestWorkspace;

    impl crate::domain::ports::LocalWorkspacePort for TestWorkspace {
        fn is_yaks_gitignored(&self) -> Result<bool> {
            Ok(true)
        }

        fn add_yaks_to_gitignore(&self) -> Result<()> {
            Ok(())
        }

        fn commit_gitignore(&self) -> Result<()> {
            Ok(())
        }

        fn is_agent_session(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_application_stamps_author_on_events() {
        use crate::domain::ports::EventStore;

        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("Test Author", "test@example.com");
        let workspace = TestWorkspace;

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
            metadata.author.name, "Test Author",
            "Event should carry author from auth port"
        );
        assert_eq!(
            metadata.author.email, "test@example.com",
            "Event should carry email from auth port"
        );
    }

    #[test]
    fn test_application_create_yak_via_yak_map() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");
        let workspace = TestWorkspace;

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            None,
            &auth,
        );

        app.with_yak_map(|yak_map| {
            yak_map.add_yak("test".to_string(), None, None, None, None, vec![])?;
            Ok(())
        })
        .unwrap();

        assert!(ReadYakStore::get_yak(&storage, &YakId::from("test")).is_ok());
    }

    #[test]
    fn test_application_mutate_yak_via_yak_map() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");
        let workspace = TestWorkspace;

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            None,
            &auth,
        );

        // Create yak and mutate its state via YakMap
        app.with_yak_map(|yak_map| {
            let id = yak_map.add_yak("test".to_string(), None, None, None, None, vec![])?;
            yak_map.update_state(id, "wip".to_string())
        })
        .unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "test").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, crate::domain::YakState::Wip);
    }

    #[test]
    fn test_application_with_yak_map() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");
        let workspace = TestWorkspace;

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            None,
            &auth,
        );

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
        assert_eq!(yak.state, crate::domain::YakState::Todo);
        assert_eq!(yak.context, Some("context".to_string()));
    }

    #[test]
    fn test_application_with_yak_map_hierarchy() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");
        let workspace = TestWorkspace;

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            None,
            &auth,
        );

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
    fn test_application_uses_event_reader_when_available() {
        use crate::domain::ports::{EventStore, EventStoreReader};

        let mut event_store = InMemoryEventStore::new();
        let mut event_store_reader = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");
        let workspace = TestWorkspace;

        // First create some events without event reader
        let mut app_no_reader = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            None,
            &auth,
        );

        app_no_reader
            .with_yak_map(|yak_map| {
                let parent_id =
                    yak_map.add_yak("parent".to_string(), None, None, None, None, vec![])?;
                yak_map.add_yak(
                    "child".to_string(),
                    Some(parent_id),
                    Some("test context".to_string()),
                    None,
                    None,
                    vec![],
                )?;
                Ok(())
            })
            .unwrap();

        // Copy events to the reader for testing
        let all_events = EventStore::get_all_events(&event_store).unwrap();
        for event in &all_events {
            event_store_reader.append(event).unwrap();
        }

        // Now create a new application with event reader and verify it can replay events
        let mut app_with_reader = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            Some(&event_store_reader as &dyn EventStoreReader),
            &auth,
        );

        // Add another yak through event replay
        app_with_reader
            .with_yak_map(|yak_map| {
                // If replay worked, we should be able to add a child under parent
                let parent_id = ReadYakStore::fuzzy_find_yak_id(&storage, "parent")?;
                yak_map.add_yak(
                    "another child".to_string(),
                    Some(parent_id),
                    None,
                    None,
                    None,
                    vec![],
                )?;
                Ok(())
            })
            .unwrap();

        // Verify state by reading from storage (which was updated by event bus)
        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 3); // parent + child + another child

        // Verify events were persisted
        let events = EventStore::get_all_events(&event_store).unwrap();
        assert!(events.len() >= 3); // Should have at least 3 Added events
    }

    #[test]
    fn test_application_with_yak_map_state_propagation() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = TestAuth::new("test", "test@test.com");
        let workspace = TestWorkspace;

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            None,
            &auth,
        );

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
        assert_eq!(parent.state, crate::domain::YakState::Wip);
    }
}
