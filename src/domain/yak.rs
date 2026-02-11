// Yak domain model

use crate::domain::YakEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yak {
    pub name: String,
    pub state: String,
    pub context: Option<String>,
    pub pending_events: Vec<YakEvent>,
}

impl Yak {
    pub fn new(name: String) -> Self {
        let mut yak = Self {
            name: name.clone(),
            state: "todo".to_string(),
            context: None,
            pending_events: vec![],
        };

        yak.pending_events.push(YakEvent::Added { name });
        yak
    }

    pub fn is_done(&self) -> bool {
        self.state == "done"
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_state(mut self, state: String) -> Self {
        self.state = state;
        self
    }

    pub fn update_context(&mut self, content: String) -> anyhow::Result<()> {
        self.context = Some(content.clone());
        self.pending_events.push(YakEvent::ContextUpdated {
            name: self.name.clone(),
            content,
        });
        Ok(())
    }

    pub fn update_state(&mut self, state: String) -> anyhow::Result<()> {
        self.state = state.clone();
        self.pending_events.push(YakEvent::StateUpdated {
            name: self.name.clone(),
            state,
        });
        Ok(())
    }

    pub fn take_events(&mut self) -> Vec<YakEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn move_to(&mut self, new_name: String) -> anyhow::Result<()> {
        validate_yak_name(&new_name).map_err(|e| anyhow::anyhow!(e))?;

        let old_name = self.name.clone();
        self.name = new_name.clone();

        self.pending_events.push(YakEvent::Moved {
            old_name,
            new_name,
        });
        Ok(())
    }

    pub fn update_field(&mut self, field_name: String, content: String) -> anyhow::Result<()> {
        self.pending_events.push(YakEvent::FieldUpdated {
            name: self.name.clone(),
            field_name,
            content,
        });
        Ok(())
    }
}

/// Validate a yak name
/// Rejects names containing forbidden characters: \ : * ? | < > "
/// Slashes (/) are allowed for hierarchical yaks (e.g., "dx/rust")
pub fn validate_yak_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Yak name cannot be empty".to_string());
    }

    // Check for forbidden characters (matches bash version)
    // Forbidden: \ : * ? | < > "
    // Allowed: / (for hierarchy)
    const FORBIDDEN_CHARS: &[char] = &['\\', ':', '*', '?', '|', '<', '>', '"'];

    for c in FORBIDDEN_CHARS {
        if name.contains(*c) {
            return Err(
                "Invalid yak name: contains forbidden characters (\\ : * ? | < > \")".to_string(),
            );
        }
    }

    Ok(())
}

/// Parse hierarchy from yak name (e.g., "dx/rust" -> ["dx", "rust"])
#[allow(dead_code)]
pub fn parse_hierarchy(name: &str) -> Vec<&str> {
    name.split('/').collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::YakEvent;

    #[test]
    fn test_new_yak() {
        let yak = Yak::new("test".to_string());
        assert_eq!(yak.name, "test");
        assert!(!yak.is_done());
        assert_eq!(yak.state, "todo");
        assert_eq!(yak.context, None);
    }

    #[test]
    fn test_yak_with_context() {
        let yak = Yak::new("test".to_string()).with_context("Some context".to_string());
        assert_eq!(yak.context, Some("Some context".to_string()));
    }

    #[test]
    fn test_mark_done() {
        let mut yak = Yak::new("test".to_string());
        yak.update_state("done".to_string()).unwrap();
        assert!(yak.is_done());
    }

    #[test]
    fn test_mark_undone() {
        let mut yak = Yak::new("test".to_string());
        yak.update_state("done".to_string()).unwrap();
        yak.update_state("todo".to_string()).unwrap();
        assert!(!yak.is_done());
    }

    #[test]
    fn test_validate_yak_name_valid() {
        assert!(validate_yak_name("test").is_ok());
        assert!(validate_yak_name("dx/rust").is_ok());
    }

    #[test]
    fn test_validate_yak_name_empty() {
        assert!(validate_yak_name("").is_err());
    }

    #[test]
    fn test_validate_yak_name_forbidden_chars() {
        // Test each forbidden character
        assert!(validate_yak_name("test\\name").is_err());
        assert!(validate_yak_name("test:name").is_err());
        assert!(validate_yak_name("test*name").is_err());
        assert!(validate_yak_name("test?name").is_err());
        assert!(validate_yak_name("test|name").is_err());
        assert!(validate_yak_name("test<name").is_err());
        assert!(validate_yak_name("test>name").is_err());
        assert!(validate_yak_name("test\"name").is_err());

        // Slash should be allowed (for hierarchy)
        assert!(validate_yak_name("test/name").is_ok());
    }

    #[test]
    fn test_parse_hierarchy() {
        assert_eq!(parse_hierarchy("dx/rust"), vec!["dx", "rust"]);
        assert_eq!(parse_hierarchy("simple"), vec!["simple"]);
        assert_eq!(parse_hierarchy("a/b/c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_yak_emits_added_event() {
        let mut yak = Yak::new("test".to_string());
        let events = yak.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Added { name } => assert_eq!(name, "test"),
            _ => panic!("Expected Added event"),
        }
    }

    #[test]
    fn test_yak_is_done_derived_from_state() {
        let mut yak = Yak::new("test".to_string());
        assert!(!yak.is_done());

        yak.state = "done".to_string();
        assert!(yak.is_done());
    }

    #[test]
    fn test_yak_update_context_emits_event() {
        let mut yak = Yak::new("test".to_string());
        yak.take_events(); // clear creation event

        yak.update_context("new context".to_string()).unwrap();
        let events = yak.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::ContextUpdated { name, content } => {
                assert_eq!(name, "test");
                assert_eq!(content, "new context");
            }
            _ => panic!("Expected ContextUpdated event"),
        }
    }

    #[test]
    fn test_yak_update_state_emits_event() {
        let mut yak = Yak::new("test".to_string());
        yak.take_events(); // clear creation event

        yak.update_state("wip".to_string()).unwrap();
        let events = yak.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::StateUpdated { name, state } => {
                assert_eq!(name, "test");
                assert_eq!(state, "wip");
            }
            _ => panic!("Expected StateUpdated event"),
        }
    }
}
