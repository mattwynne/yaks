// Use case: Set a yak's state

use anyhow::Result;

use super::{Application, UseCase};

pub struct SetState {
    name: String,
    state: String,
    recursive: bool,
}

impl SetState {
    pub fn new(name: &str, state: &str) -> Self {
        Self {
            name: name.to_string(),
            state: state.to_string(),
            recursive: false,
        }
    }

    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        use crate::domain::slug::YakId;

        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        let ids_to_update = if self.recursive {
            // TODO: Move recursive descendant-finding into YakMap using parent_id
            // relationships instead of string prefix matching on display names
            let all_yaks = app.store.list_yaks()?;
            let resolved_yak = app.store.get_yak(&id)?;
            let resolved_name = resolved_yak.name.to_string();
            let mut yaks_to_update: Vec<(usize, YakId)> = all_yaks
                .iter()
                .filter(|yak| {
                    yak.name == resolved_name
                        || yak.name.as_str().starts_with(&format!("{resolved_name}/"))
                })
                .map(|yak| {
                    let depth = yak.name.as_str().matches('/').count();
                    (depth, yak.id.clone())
                })
                .collect();
            // Sort by depth descending (leaves first) so children are
            // marked done before parents, passing hierarchy validation
            yaks_to_update.sort_by(|a, b| b.0.cmp(&a.0));
            yaks_to_update.into_iter().map(|(_, id)| id).collect()
        } else {
            vec![id]
        };

        let state = self.state.clone();
        app.with_yak_map(move |yak_map| {
            for id in ids_to_update {
                yak_map.update_state(id, state.clone())?;
            }
            Ok(())
        })
    }
}

impl UseCase for SetState {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage};
    use crate::application::AddYak;
    use crate::domain::ports::ReadYakStore;
    use crate::infrastructure::EventBus;

    fn setup() -> (InMemoryStorage, InMemoryDisplay, InMemoryInput) {
        (
            InMemoryStorage::new(),
            InMemoryDisplay::new(),
            InMemoryInput::new(),
        )
    }

    #[test]
    fn sets_state_with_exact_name() {
        let (storage, display, input) = setup();
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));
        event_bus.register(Box::new(storage.clone()));
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        AddYak::new("my yak").execute(&mut app).unwrap();
        SetState::new("my yak", "wip").execute(&mut app).unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "my yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn resolves_fuzzy_name() {
        let (storage, display, input) = setup();
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));
        event_bus.register(Box::new(storage.clone()));
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        AddYak::new("Fix the bug").execute(&mut app).unwrap();
        SetState::new("bug", "wip").execute(&mut app).unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "Fix the bug").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, "wip");
    }

    #[test]
    fn errors_on_ambiguous_fuzzy_name() {
        let (storage, display, input) = setup();
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));
        event_bus.register(Box::new(storage.clone()));
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        AddYak::new("Fix the bug").execute(&mut app).unwrap();
        AddYak::new("Report the bug").execute(&mut app).unwrap();
        let result = SetState::new("bug", "wip").execute(&mut app);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ambiguous"));
    }

    #[test]
    fn sets_state_recursively() {
        let (storage, display, input) = setup();
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));
        event_bus.register(Box::new(storage.clone()));
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        // Add hierarchical yaks directly via yak_map (bypasses name validation)
        app.with_yak_map(|yak_map| {
            let parent_id = yak_map.add_yak("parent".to_string(), None, None)?;
            let child_id = yak_map.add_yak("child".to_string(), Some(parent_id), None)?;
            yak_map.add_yak("grandchild".to_string(), Some(child_id), None)?;
            Ok(())
        })
        .unwrap();

        SetState::new("parent", "done")
            .with_recursive(true)
            .execute(&mut app)
            .unwrap();

        let parent_id = ReadYakStore::fuzzy_find_yak_id(&storage, "parent").unwrap();
        let parent = ReadYakStore::get_yak(&storage, &parent_id).unwrap();
        // Find child by listing and filtering (child is nested under parent)
        let all_yaks = ReadYakStore::list_yaks(&storage).unwrap();
        let child = all_yaks.iter().find(|y| y.name == "parent/child").unwrap();
        let grandchild = all_yaks
            .iter()
            .find(|y| y.name == "parent/child/grandchild")
            .unwrap();
        assert_eq!(parent.state, "done");
        assert_eq!(child.state, "done");
        assert_eq!(grandchild.state, "done");
    }

    #[test]
    fn errors_on_not_found() {
        let (storage, display, input) = setup();
        let event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new(Box::new(event_store));
        event_bus.register(Box::new(storage.clone()));
        let mut app = Application::new(&mut event_bus, &storage, &display, &input, None, None);

        let result = SetState::new("nonexistent", "wip").execute(&mut app);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
