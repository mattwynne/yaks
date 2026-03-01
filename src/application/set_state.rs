// Use case: Set a yak's state

use anyhow::Result;

use crate::domain::slug::YakId;

use super::{Application, UseCase};

pub struct SetState {
    name: String,
    state: String,
    recursive: bool,
    silent: bool,
}

impl SetState {
    pub fn new(name: &str, state: &str) -> Self {
        Self {
            name: name.to_string(),
            state: state.to_string(),
            recursive: false,
            silent: false,
        }
    }

    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub fn with_silent(mut self, silent: bool) -> Self {
        self.silent = silent;
        self
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;

        let ids_to_update = if self.recursive {
            // Find all descendants using parent_id relationships
            let all_yaks = app.store.list_yaks()?;
            let mut descendants = vec![id.clone()];
            Self::collect_descendants(&id, &all_yaks, &mut descendants);
            // Reverse so leaves come first (children before parents)
            descendants.reverse();
            descendants
        } else {
            vec![id]
        };

        let state = self.state.clone();
        app.with_yak_map(move |yak_map| {
            for id in ids_to_update {
                yak_map.update_state(id, state.clone())?;
            }
            Ok(())
        })?;

        if !self.silent {
            app.display
                .success(&format!("Set '{}' state to {}", self.name, self.state));
        }

        Ok(())
    }

    /// Recursively collect all descendant IDs (breadth-first, parents before children).
    fn collect_descendants(
        parent_id: &YakId,
        all_yaks: &[crate::domain::YakView],
        result: &mut Vec<YakId>,
    ) {
        let children: Vec<&crate::domain::YakView> = all_yaks
            .iter()
            .filter(|yak| yak.parent_id.as_ref() == Some(parent_id))
            .collect();
        for child in children {
            result.push(child.id.clone());
            Self::collect_descendants(&child.id, all_yaks, result);
        }
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
    use crate::adapters::user_display::ConsoleDisplay;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::AddYak;
    use crate::domain::ports::ReadYakStore;
    use crate::infrastructure::EventBus;

    fn setup() -> (
        InMemoryStorage,
        ConsoleDisplay,
        crate::adapters::TestBuffer,
        InMemoryInput,
    ) {
        let (display, buffer) = make_test_display();
        (
            InMemoryStorage::new(),
            display,
            buffer,
            InMemoryInput::new(),
        )
    }

    #[test]
    fn sets_state_with_exact_name() {
        let (storage, display, buffer, input) = setup();
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        event_bus.register(Box::new(storage.clone()));
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

        AddYak::new("my yak").execute(&mut app).unwrap();
        SetState::new("my yak", "wip").execute(&mut app).unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "my yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, "wip");
        assert!(buffer.contents().contains("Set 'my yak' state to wip"));
    }

    #[test]
    fn resolves_fuzzy_name() {
        let (storage, display, buffer, input) = setup();
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        event_bus.register(Box::new(storage.clone()));
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

        AddYak::new("Fix the bug").execute(&mut app).unwrap();
        SetState::new("bug", "wip").execute(&mut app).unwrap();

        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "Fix the bug").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, "wip");
        assert!(buffer.contents().contains("Set 'bug' state to wip"));
    }

    #[test]
    fn errors_on_ambiguous_fuzzy_name() {
        let (storage, display, _buffer, input) = setup();
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        event_bus.register(Box::new(storage.clone()));
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

        AddYak::new("Fix the bug").execute(&mut app).unwrap();
        AddYak::new("Report the bug").execute(&mut app).unwrap();
        let result = SetState::new("bug", "wip").execute(&mut app);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ambiguous"));
    }

    #[test]
    fn sets_state_recursively() {
        let (storage, display, buffer, input) = setup();
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        event_bus.register(Box::new(storage.clone()));
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

        // Add hierarchical yaks directly via yak_map (bypasses name validation)
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
            yak_map.add_yak(
                "grandchild".to_string(),
                Some(child_id),
                None,
                None,
                None,
                vec![],
            )?;
            Ok(())
        })
        .unwrap();

        SetState::new("parent", "done")
            .with_recursive(true)
            .execute(&mut app)
            .unwrap();

        let parent_id = ReadYakStore::fuzzy_find_yak_id(&storage, "parent").unwrap();
        let parent = ReadYakStore::get_yak(&storage, &parent_id).unwrap();
        // Find child by listing and filtering (stores return leaf names)
        let all_yaks = ReadYakStore::list_yaks(&storage).unwrap();
        let child = all_yaks.iter().find(|y| y.name == "child").unwrap();
        let grandchild = all_yaks.iter().find(|y| y.name == "grandchild").unwrap();
        assert_eq!(parent.state, "done");
        assert_eq!(child.state, "done");
        assert_eq!(grandchild.state, "done");
        assert!(buffer.contents().contains("Set 'parent' state to done"));
    }

    #[test]
    fn errors_on_not_found() {
        let (storage, display, _buffer, input) = setup();
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        event_bus.register(Box::new(storage.clone()));
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

        let result = SetState::new("nonexistent", "wip").execute(&mut app);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
