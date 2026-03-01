// InProcessWorld - calls Application directly with in-memory adapters

use anyhow::{Context, Result};
use cucumber::World as CucumberWorld;
use std::collections::HashMap;

use super::test_world::TestWorld;
use yx::adapters::user_display::ConsoleDisplay;
use yx::adapters::{
    make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput, InMemoryStorage,
    TestBuffer,
};
use yx::application::{
    AddTag, AddYak, Application, DoneYak, EditContext, ListTags, ListYaks, MoveYak, PruneYaks,
    RemoveTag, RemoveYak, RenameYak, SetState, ShowContext, ShowField, StartYak, SyncYaks,
    WriteField,
};
use yx::domain::normalize_tag;
use yx::domain::ports::EventStore;
use yx::infrastructure::EventBus;

/// A named user instance for multi-repo sync scenarios.
/// Each user (alice, bob) gets their own set of in-memory adapters.
struct UserInstance {
    event_store: InMemoryEventStore,
    event_bus: EventBus,
    storage: InMemoryStorage,
    display: ConsoleDisplay,
    buffer: TestBuffer,
    input: InMemoryInput,
    auth: InMemoryAuthentication,
}

impl UserInstance {
    fn new() -> Self {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();

        Self {
            event_store,
            event_bus,
            storage,
            display,
            buffer,
            input: InMemoryInput::new(),
            auth: InMemoryAuthentication::new(),
        }
    }
}

#[derive(CucumberWorld)]
#[world(init = Self::new)]
pub struct InProcessWorld {
    event_store: InMemoryEventStore,
    event_bus: EventBus,
    storage: InMemoryStorage,
    display: ConsoleDisplay,
    buffer: TestBuffer,
    input: InMemoryInput,
    auth: InMemoryAuthentication,
    error: String,
    exit_code: i32,
    /// Named user instances for multi-repo sync scenarios
    repos: HashMap<String, UserInstance>,
    /// Shared "origin" event store for sync scenarios
    origin: Option<InMemoryEventStore>,
}

impl std::fmt::Debug for InProcessWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessWorld")
            .field("exit_code", &self.exit_code)
            .finish()
    }
}

impl InProcessWorld {
    fn new() -> Result<Self> {
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, buffer) = make_test_display();

        Ok(Self {
            event_store,
            event_bus,
            storage,
            display,
            buffer,
            input: InMemoryInput::new(),
            auth: InMemoryAuthentication::new(),
            error: String::new(),
            exit_code: 0,
            repos: HashMap::new(),
            origin: None,
        })
    }

    fn execute<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Application) -> Result<()>,
    {
        self.buffer.clear();
        self.error.clear();

        let mut app = Application::new(
            &mut self.event_store,
            &mut self.event_bus,
            &self.storage,
            &self.display,
            &self.input,
            None,
            &self.auth,
        );
        let result = f(&mut app);

        self.exit_code = if result.is_ok() { 0 } else { 1 };

        result
    }

    fn try_execute<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Application) -> Result<()>,
    {
        self.buffer.clear();
        self.error.clear();

        let mut app = Application::new(
            &mut self.event_store,
            &mut self.event_bus,
            &self.storage,
            &self.display,
            &self.input,
            None,
            &self.auth,
        );
        let result = f(&mut app);

        match result {
            Ok(()) => self.exit_code = 0,
            Err(e) => {
                self.exit_code = 1;
                self.error = e.to_string();
            }
        }

        Ok(())
    }

    /// Create a bare "origin" event store (in-memory equivalent of a bare git repo)
    pub fn create_bare_repo(&mut self, name: &str) -> Result<()> {
        if name == "origin" {
            self.origin = Some(InMemoryEventStore::new());
        }
        // For non-origin bare repos, store as a regular repo
        // (not needed for current scenarios but keeps API consistent)
        Ok(())
    }

    /// Create a "clone" of origin -- a new user instance that can sync with origin
    pub fn create_clone(&mut self, _origin_name: &str, clone_name: &str) -> Result<()> {
        self.repos
            .insert(clone_name.to_string(), UserInstance::new());
        Ok(())
    }

    /// Execute a command in a named user's context
    pub fn execute_in_repo<F>(&mut self, repo_name: &str, f: F) -> Result<()>
    where
        F: FnOnce(&mut Application) -> Result<()>,
    {
        let user = self
            .repos
            .get_mut(repo_name)
            .context(format!("No repo named '{}'", repo_name))?;

        user.buffer.clear();

        let mut app = Application::new(
            &mut user.event_store,
            &mut user.event_bus,
            &user.storage,
            &user.display,
            &user.input,
            None,
            &user.auth,
        );
        let result = f(&mut app);

        self.exit_code = if result.is_ok() { 0 } else { 1 };
        self.error = match &result {
            Ok(()) => String::new(),
            Err(e) => e.to_string(),
        };

        result
    }

    /// Sync a named user's event store with origin
    pub fn sync_repo(&mut self, repo_name: &str) -> Result<()> {
        let origin = self.origin.as_ref().context("No origin configured")?;

        let user = self
            .repos
            .get_mut(repo_name)
            .context(format!("No repo named '{}'", repo_name))?;

        // Temporarily swap in a peer-configured event store for sync
        let mut syncing_store = InMemoryEventStore::with_peer(origin);
        // Copy existing events into the syncing store
        for event in yx::domain::ports::EventStore::get_all_events(&user.event_store)? {
            syncing_store.append(&event)?;
        }

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let storage = user.storage.clone();

        {
            let mut app = Application::new(
                &mut syncing_store,
                &mut user.event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            app.handle(SyncYaks::new())?;
        }

        // Replace user's event store with the synced one
        user.event_store = syncing_store;

        self.exit_code = 0;
        Ok(())
    }

    /// Get output from a named user's display
    pub fn get_repo_output(&self, repo_name: &str) -> Result<String> {
        let user = self
            .repos
            .get(repo_name)
            .context(format!("No repo named '{}'", repo_name))?;
        Ok(user.buffer.contents().trim_end().to_string())
    }

    /// List yaks in a named user's store
    pub fn list_yaks_in_repo(&self, repo_name: &str) -> Result<Vec<yx::domain::YakView>> {
        let user = self
            .repos
            .get(repo_name)
            .context(format!("No repo named '{}'", repo_name))?;
        yx::domain::ports::ReadYakStore::list_yaks(&user.storage)
    }

    /// Set input content for a named user (for context/field commands)
    pub fn set_input_in_repo(&mut self, repo_name: &str, content: &str) -> Result<()> {
        let user = self
            .repos
            .get_mut(repo_name)
            .context(format!("No repo named '{}'", repo_name))?;
        user.input.set_content(Some(content.to_string()));
        Ok(())
    }
}

impl TestWorld for InProcessWorld {
    fn add_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(AddYak::new(name)))
    }

    fn add_yak_under(&mut self, name: &str, parent: &str) -> Result<()> {
        let name = name.to_string();
        let parent = parent.to_string();
        self.execute(move |app| app.handle(AddYak::new(&name).with_parent(Some(&parent))))
    }

    fn try_add_yak(&mut self, name: &str) -> Result<()> {
        self.try_execute(|app| app.handle(AddYak::new(name)))
    }

    fn try_add_yak_under(&mut self, name: &str, parent: &str) -> Result<()> {
        let name = name.to_string();
        let parent = parent.to_string();
        self.try_execute(move |app| app.handle(AddYak::new(&name).with_parent(Some(&parent))))
    }

    fn remove_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(RemoveYak::new(name)))
    }

    fn remove_yak_recursive(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(RemoveYak::new(name).with_recursive(true)))
    }

    fn try_remove_yak(&mut self, name: &str) -> Result<()> {
        self.try_execute(|app| app.handle(RemoveYak::new(name)))
    }

    fn get_error(&self) -> String {
        self.error.clone()
    }

    fn done_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(DoneYak::new(name, false)))
    }

    fn list_yaks(&mut self) -> Result<()> {
        self.execute(|app| app.handle(ListYaks::new("pretty", None)))
    }

    fn list_yaks_with_format(&mut self, format: &str) -> Result<()> {
        self.execute(|app| app.handle(ListYaks::new(format, None)))
    }

    fn list_yaks_with_format_and_filter(&mut self, format: &str, only: &str) -> Result<()> {
        self.execute(|app| app.handle(ListYaks::new(format, Some(only))))
    }

    fn list_yaks_json(&mut self) -> Result<()> {
        self.execute(|app| app.handle(ListYaks::new("json", None)))
    }

    fn try_list_yaks_with_format(&mut self, format: &str) -> Result<()> {
        let format = format.to_string();
        self.try_execute(move |app| app.handle(ListYaks::new(&format, None)))
    }

    fn try_list_yaks_with_filter(&mut self, only: &str) -> Result<()> {
        let only = only.to_string();
        self.try_execute(move |app| app.handle(ListYaks::new("pretty", Some(&only))))
    }

    fn set_context(&mut self, name: &str, content: &str) -> Result<()> {
        self.input.set_content(Some(content.to_string()));
        self.execute(|app| app.handle(EditContext::new(name)))
    }

    fn show_context(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(ShowContext::new(name)))
    }

    fn try_done_yak(&mut self, name: &str) -> Result<()> {
        self.try_execute(|app| app.handle(DoneYak::new(name, false)))
    }

    fn done_yak_recursive(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(DoneYak::new(name, true)))
    }

    fn get_output(&self) -> String {
        self.buffer.contents().trim_end().to_string()
    }

    fn prune_yaks(&mut self) -> Result<()> {
        self.execute(|app| app.handle(PruneYaks::new()))
    }

    fn set_state(&mut self, name: &str, state: &str) -> Result<()> {
        self.execute(|app| app.handle(SetState::new(name, state)))
    }

    fn try_set_state(&mut self, name: &str, state: &str) -> Result<()> {
        self.try_execute(|app| app.handle(SetState::new(name, state)))
    }

    fn start_yak(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(StartYak::new(name, false)))
    }

    fn move_yak_under(&mut self, name: &str, parent: &str) -> Result<()> {
        self.execute(|app| app.handle(MoveYak::under(name, parent)))
    }

    fn move_yak_to_root(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(MoveYak::to_root(name)))
    }

    fn try_move_yak_under_and_to_root(&mut self, _name: &str, _parent: &str) -> Result<()> {
        // Invalid state: both flags specified. With the MoveTarget enum,
        // this is unrepresentable. Simulate the CLI-layer error.
        self.exit_code = 1;
        self.error = "Cannot use both --under and --to-root. Use one or the other.".to_string();
        Ok(())
    }

    fn try_move_yak_no_flags(&mut self, _name: &str) -> Result<()> {
        // Invalid state: no flags specified. With the MoveTarget enum,
        // this is unrepresentable. Simulate the CLI-layer error.
        self.exit_code = 1;
        self.error = "Must specify either --under <parent> or --to-root.".to_string();
        Ok(())
    }

    fn try_move_yak_under(&mut self, name: &str, parent: &str) -> Result<()> {
        self.try_execute(|app| app.handle(MoveYak::under(name, parent)))
    }

    fn set_field(&mut self, name: &str, field: &str, content: &str) -> Result<()> {
        self.input.set_content(Some(content.to_string()));
        self.execute(|app| app.handle(WriteField::new(name, field)))
    }

    fn try_set_field(&mut self, name: &str, field: &str, content: &str) -> Result<()> {
        self.input.set_content(Some(content.to_string()));
        self.try_execute(|app| app.handle(WriteField::new(name, field)))
    }

    fn show_field(&mut self, name: &str, field: &str) -> Result<()> {
        self.execute(|app| app.handle(ShowField::new(name, field)))
    }

    fn rename_yak(&mut self, from: &str, to: &str) -> Result<()> {
        self.execute(|app| app.handle(RenameYak::new(from, to)))
    }

    fn try_rename_yak(&mut self, from: &str, to: &str) -> Result<()> {
        self.try_execute(|app| app.handle(RenameYak::new(from, to)))
    }

    fn add_yak_with_state(&mut self, name: &str, state: &str) -> Result<()> {
        let name = name.to_string();
        let state = state.to_string();
        self.execute(move |app| app.handle(AddYak::new(&name).with_state(Some(&state))))
    }

    fn add_yak_with_context(&mut self, name: &str, context: &str) -> Result<()> {
        let name = name.to_string();
        let context = context.to_string();
        self.execute(move |app| app.handle(AddYak::new(&name).with_context(Some(&context))))
    }

    fn add_yak_with_id(&mut self, name: &str, id: &str) -> Result<()> {
        let name = name.to_string();
        let id = id.to_string();
        self.execute(move |app| app.handle(AddYak::new(&name).with_id(Some(&id))))
    }

    fn add_yak_with_field(&mut self, name: &str, key: &str, value: &str) -> Result<()> {
        let name = name.to_string();
        let key = key.to_string();
        let value = value.to_string();
        self.execute(move |app| app.handle(AddYak::new(&name).with_field(&key, &value)))
    }

    fn add_tags(&mut self, name: &str, tags: Vec<String>) -> Result<()> {
        let normalized: Vec<String> = tags
            .iter()
            .map(|t| normalize_tag(t))
            .collect::<Result<_>>()?;
        let name = name.to_string();
        self.execute(move |app| app.handle(AddTag::new(&name, normalized)))
    }

    fn remove_tags(&mut self, name: &str, tags: Vec<String>) -> Result<()> {
        let normalized: Vec<String> = tags
            .iter()
            .map(|t| normalize_tag(t))
            .collect::<Result<_>>()?;
        let name = name.to_string();
        self.execute(move |app| app.handle(RemoveTag::new(&name, normalized)))
    }

    fn list_tags(&mut self, name: &str) -> Result<()> {
        self.execute(|app| app.handle(ListTags::new(name)))
    }

    fn create_bare_repo(&mut self, name: &str) -> Result<()> {
        InProcessWorld::create_bare_repo(self, name)
    }

    fn create_clone(&mut self, origin: &str, clone: &str) -> Result<()> {
        InProcessWorld::create_clone(self, origin, clone)
    }

    fn get_exit_code(&self) -> i32 {
        self.exit_code
    }
}
