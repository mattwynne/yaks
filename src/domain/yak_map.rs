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

    pub fn update_state(&mut self, name: String, state: String) -> Result<()> {
        use crate::domain::validate_state;

        validate_state(&state).map_err(|e| anyhow::anyhow!(e))?;

        if !self.yaks.contains_key(&name) {
            anyhow::bail!("Yak '{}' not found", name);
        }

        // Validate children if marking done
        if state == "done" {
            self.validate_children_complete(&name)?;
        }

        // Capture old state before updating
        let old_state = self.yaks.get(&name).unwrap().state.clone();
        let transitioning_from_todo = old_state == "todo" && state != "todo";

        // Update this yak
        self.yaks.get_mut(&name).unwrap().state = state.clone();
        self.pending_events.push(YakEvent::StateUpdated {
            name: name.clone(),
            state
        });

        // Propagate to ancestors if transitioning from todo
        if transitioning_from_todo {
            self.propagate_wip_to_ancestors(&name);
        }

        Ok(())
    }

    fn validate_children_complete(&self, parent_name: &str) -> Result<()> {
        use crate::domain::find_children;

        let children = find_children(parent_name, &self.yaks);

        if !children.is_empty() {
            let incomplete = children.iter()
                .any(|name| self.yaks.get(name).unwrap().state != "done");

            if incomplete {
                anyhow::bail!(
                    "Cannot mark '{}' as done: children are incomplete",
                    parent_name
                );
            }
        }

        Ok(())
    }

    fn propagate_wip_to_ancestors(&mut self, child_name: &str) {
        use crate::domain::get_ancestors;

        for ancestor in get_ancestors(child_name) {
            if let Some(parent) = self.yaks.get_mut(&ancestor) {
                if parent.state == "todo" {
                    parent.state = "wip".to_string();
                    self.pending_events.push(YakEvent::StateUpdated {
                        name: ancestor,
                        state: "wip".to_string(),
                    });
                }
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

    // Tests for update_state
    #[test]
    fn test_update_state_changes_state() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None).unwrap();
        map.take_events();
        map.update_state("test".to_string(), "wip".to_string()).unwrap();
        assert_eq!(map.yaks.get("test").unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_validates_state() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None).unwrap();
        let result = map.update_state("test".to_string(), "invalid".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_update_state_prevents_marking_parent_done_with_incomplete_children() {
        let mut map = YakMap::new();
        map.add_yak("parent".to_string(), None).unwrap();
        map.add_yak("parent/child".to_string(), None).unwrap();
        let result = map.update_state("parent".to_string(), "done".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("children are incomplete"));
    }

    #[test]
    fn test_update_state_allows_marking_parent_done_with_all_children_done() {
        let mut map = YakMap::new();
        map.add_yak("parent".to_string(), None).unwrap();
        map.add_yak("parent/child".to_string(), None).unwrap();
        map.update_state("parent/child".to_string(), "done".to_string()).unwrap();
        let result = map.update_state("parent".to_string(), "done".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_state_propagates_to_parent_on_todo_transition() {
        let mut map = YakMap::new();
        map.add_yak("parent".to_string(), None).unwrap();
        map.add_yak("parent/child".to_string(), None).unwrap();
        map.take_events();
        map.update_state("parent/child".to_string(), "wip".to_string()).unwrap();
        assert_eq!(map.yaks.get("parent").unwrap().state, "wip");
        assert_eq!(map.yaks.get("parent/child").unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_propagates_through_multiple_levels() {
        let mut map = YakMap::new();
        map.add_yak("a/b/c".to_string(), None).unwrap();
        map.take_events();
        map.update_state("a/b/c".to_string(), "wip".to_string()).unwrap();
        assert_eq!(map.yaks.get("a").unwrap().state, "wip");
        assert_eq!(map.yaks.get("a/b").unwrap().state, "wip");
        assert_eq!(map.yaks.get("a/b/c").unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_only_propagates_on_todo_transition() {
        let mut map = YakMap::new();
        map.add_yak("parent".to_string(), None).unwrap();
        map.add_yak("parent/child".to_string(), None).unwrap();
        map.update_state("parent/child".to_string(), "wip".to_string()).unwrap();
        map.take_events();
        map.update_state("parent/child".to_string(), "done".to_string()).unwrap();
        let events = map.take_events();
        assert_eq!(events.len(), 1); // Only child event
    }
}
