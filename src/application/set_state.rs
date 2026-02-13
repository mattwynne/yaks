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

    fn resolve_name(&self, app: &Application) -> Result<String> {
        let all_yaks = app.store.list_yaks()?;
        let name = &self.name;

        if app.store.yak_exists(name) {
            return Ok(name.clone());
        }

        let matches: Vec<String> = all_yaks
            .iter()
            .filter(|yak| {
                let leaf = yak.name.rsplit('/').next().unwrap_or(&yak.name);
                leaf.contains(name.as_str())
            })
            .map(|yak| yak.name.clone())
            .collect();

        match matches.len() {
            0 => anyhow::bail!("yak '{}' not found", name),
            1 => Ok(matches[0].clone()),
            _ => anyhow::bail!("yak name '{}' is ambiguous", name),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let resolved_name = self.resolve_name(app)?;

        let names_to_update = if self.recursive {
            let all_yaks = app.store.list_yaks()?;
            let mut names: Vec<String> = all_yaks
                .iter()
                .filter(|yak| {
                    yak.name == resolved_name || yak.name.starts_with(&format!("{resolved_name}/"))
                })
                .map(|yak| yak.name.clone())
                .collect();
            // Sort by depth descending (leaves first) so children are
            // marked done before parents, passing hierarchy validation
            names.sort_by(|a, b| {
                let depth_a = a.matches('/').count();
                let depth_b = b.matches('/').count();
                depth_b.cmp(&depth_a)
            });
            names
        } else {
            vec![resolved_name]
        };

        let state = self.state.clone();
        app.with_yak_map(move |yak_map| {
            for name in names_to_update {
                yak_map.update_state(name, state.clone())?;
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
    use crate::infrastructure::EventBus;
    use crate::ports::Store;

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

        let yak = Store::get_yak(&storage, "my yak").unwrap();
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

        let yak = Store::get_yak(&storage, "Fix the bug").unwrap();
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

        AddYak::new("parent").execute(&mut app).unwrap();
        AddYak::new("parent/child").execute(&mut app).unwrap();
        AddYak::new("parent/child/grandchild")
            .execute(&mut app)
            .unwrap();

        SetState::new("parent", "done")
            .with_recursive(true)
            .execute(&mut app)
            .unwrap();

        let parent = Store::get_yak(&storage, "parent").unwrap();
        let child = Store::get_yak(&storage, "parent/child").unwrap();
        let grandchild = Store::get_yak(&storage, "parent/child/grandchild").unwrap();
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
