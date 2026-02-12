use crate::domain::YakEvent;
#[allow(unused_imports)]
use crate::ports::Store;
#[allow(unused_imports)]
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YakState {
    pub(crate) state: String,
    pub(crate) context: Option<String>,
}

pub struct YakMap {
    yaks: HashMap<String, YakState>,
    pending_events: Vec<YakEvent>,
}

impl YakMap {
    pub fn new() -> Self {
        Self {
            yaks: HashMap::new(),
            pending_events: Vec::new(),
        }
    }

    pub fn take_events(&mut self) -> Vec<YakEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn add_yak(&mut self, name: String, context: Option<String>) -> Result<()> {
        use crate::domain::validate_yak_name;

        validate_yak_name(&name).map_err(|e| anyhow::anyhow!(e))?;

        // Ensure all ancestors exist
        self.ensure_ancestors_exist(&name);

        self.yaks.insert(name.clone(), YakState {
            state: "todo".to_string(),
            context: context.clone(),
        });

        self.pending_events.push(YakEvent::Added { name: name.clone() });

        if let Some(content) = context {
            self.pending_events.push(YakEvent::ContextUpdated { name, content });
        }

        Ok(())
    }

    fn ensure_ancestors_exist(&mut self, name: &str) {
        use crate::domain::get_ancestors;

        for ancestor in get_ancestors(name) {
            if !self.yaks.contains_key(&ancestor) {
                self.yaks.insert(ancestor.clone(), YakState {
                    state: "todo".to_string(),
                    context: None,
                });
                self.pending_events.push(YakEvent::Added { name: ancestor });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_yak_map_is_empty() {
        let map = YakMap::new();
        assert_eq!(map.yaks.len(), 0);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_take_events_removes_events() {
        let mut map = YakMap::new();
        map.pending_events.push(YakEvent::Added {
            name: "test".to_string()
        });

        let events = map.take_events();

        assert_eq!(events.len(), 1);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_add_yak_creates_yak_with_todo_state() {
        let mut map = YakMap::new();

        map.add_yak("test".to_string(), None).unwrap();

        assert!(map.yaks.contains_key("test"));
        assert_eq!(map.yaks.get("test").unwrap().state, "todo");
        assert_eq!(map.yaks.get("test").unwrap().context, None);
    }

    #[test]
    fn test_add_yak_with_context() {
        let mut map = YakMap::new();

        map.add_yak("test".to_string(), Some("context".to_string())).unwrap();

        assert_eq!(map.yaks.get("test").unwrap().context, Some("context".to_string()));
    }

    #[test]
    fn test_add_yak_emits_added_event() {
        let mut map = YakMap::new();

        map.add_yak("test".to_string(), None).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Added { name } => assert_eq!(name, "test"),
            _ => panic!("Expected Added event"),
        }
    }

    #[test]
    fn test_add_yak_with_context_emits_two_events() {
        let mut map = YakMap::new();

        map.add_yak("test".to_string(), Some("context".to_string())).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 2);
        match &events[0] {
            YakEvent::Added { name } => assert_eq!(name, "test"),
            _ => panic!("Expected Added event first"),
        }
        match &events[1] {
            YakEvent::ContextUpdated { name, content } => {
                assert_eq!(name, "test");
                assert_eq!(content, "context");
            }
            _ => panic!("Expected ContextUpdated event second"),
        }
    }

    #[test]
    fn test_add_yak_auto_creates_single_ancestor() {
        let mut map = YakMap::new();

        map.add_yak("parent/child".to_string(), None).unwrap();

        assert!(map.yaks.contains_key("parent"));
        assert_eq!(map.yaks.get("parent").unwrap().state, "todo");
        assert_eq!(map.yaks.get("parent").unwrap().context, None);
    }

    #[test]
    fn test_add_yak_auto_creates_multiple_ancestors() {
        let mut map = YakMap::new();

        map.add_yak("a/b/c".to_string(), None).unwrap();

        assert!(map.yaks.contains_key("a"));
        assert!(map.yaks.contains_key("a/b"));
        assert!(map.yaks.contains_key("a/b/c"));
    }

    #[test]
    fn test_add_yak_emits_events_for_ancestors() {
        let mut map = YakMap::new();

        map.add_yak("a/b/c".to_string(), None).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 3); // Added(a), Added(a/b), Added(a/b/c)
    }

    #[test]
    fn test_add_yak_doesnt_recreate_existing_ancestor() {
        let mut map = YakMap::new();

        map.add_yak("parent".to_string(), Some("context".to_string())).unwrap();
        map.take_events(); // Clear events

        map.add_yak("parent/child".to_string(), None).unwrap();

        // Parent context should be preserved
        assert_eq!(map.yaks.get("parent").unwrap().context, Some("context".to_string()));

        // Only one Added event (for child)
        let events = map.take_events();
        assert_eq!(events.len(), 1);
    }
}
