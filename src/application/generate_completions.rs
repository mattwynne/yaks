// GenerateCompletions use case - generates shell completion suggestions

use anyhow::Result;

use super::completions::complete_with_state;
use super::{Application, UseCase};

pub struct GenerateCompletions {
    words: Vec<String>,
}

impl GenerateCompletions {
    pub fn new(words: Vec<String>) -> Self {
        Self { words }
    }
}

impl UseCase for GenerateCompletions {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let yaks = app.store.list_yaks()?;

        let yak_name_strings: Vec<String> = yaks.iter().map(|y| y.name.to_string()).collect();
        let yaks_with_state: Vec<(&str, bool)> = yak_name_strings
            .iter()
            .zip(yaks.iter())
            .map(|(name, yak)| (name.as_str(), yak.is_done()))
            .collect();

        let word_refs: Vec<&str> = self.words.iter().map(|s| s.as_str()).collect();

        let results = complete_with_state(&word_refs, &yaks_with_state);

        for result in results {
            app.display.info(&result);
        }

        Ok(())
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
    use crate::application::{AddYak, SetState};
    use crate::infrastructure::EventBus;

    fn make_app<'a>(
        event_store: &'a mut InMemoryEventStore,
        event_bus: &'a mut EventBus,
        storage: &'a InMemoryStorage,
        display: &'a ConsoleDisplay,
        input: &'a InMemoryInput,
        auth: &'a InMemoryAuthentication,
    ) -> Application<'a> {
        Application::new(event_store, event_bus, storage, display, input, None, auth)
    }

    #[test]
    fn completes_subcommands() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        app.handle(GenerateCompletions::new(vec![
            "yx".to_string(),
            "".to_string(),
        ]))
        .unwrap();

        let output = buffer.contents();
        assert!(output.contains("add"), "Should include 'add' command");
        assert!(output.contains("list"), "Should include 'list' command");
        assert!(output.contains("done"), "Should include 'done' command");
    }

    #[test]
    fn completes_yak_names_for_done_command() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        app.handle(AddYak::new("fix bug")).unwrap();
        app.handle(AddYak::new("write docs")).unwrap();
        buffer.clear();

        app.handle(GenerateCompletions::new(vec![
            "yx".to_string(),
            "done".to_string(),
            "".to_string(),
        ]))
        .unwrap();

        let output = buffer.contents();
        assert!(
            output.contains("fix bug"),
            "Should include 'fix bug' yak name"
        );
        assert!(
            output.contains("write docs"),
            "Should include 'write docs' yak name"
        );
    }

    #[test]
    fn done_command_excludes_done_yaks() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        app.handle(AddYak::new("pending")).unwrap();
        app.handle(AddYak::new("finished")).unwrap();
        app.handle(SetState::new("finished", "done")).unwrap();
        buffer.clear();

        app.handle(GenerateCompletions::new(vec![
            "yx".to_string(),
            "done".to_string(),
            "".to_string(),
        ]))
        .unwrap();

        let output = buffer.contents();
        assert!(
            output.contains("pending"),
            "Should include incomplete yak 'pending'"
        );
        assert!(
            !output.contains("finished"),
            "Should exclude done yak 'finished'"
        );
    }
}
