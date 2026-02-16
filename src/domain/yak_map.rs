use crate::domain::events::*;
use crate::domain::ports::ReadYakStore;
use crate::domain::slug::generate_slug;
use crate::domain::YakEvent;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YakState {
    pub(crate) id: String,
    pub(crate) state: String,
    pub(crate) context: Option<String>,
}

pub struct YakMap {
    yaks: HashMap<String, YakState>,
    pending_events: Vec<YakEvent>,
}

impl YakMap {
    #[cfg(test)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            yaks: HashMap::new(),
            pending_events: Vec::new(),
        }
    }

    pub fn from_store(store: &dyn ReadYakStore) -> Result<Self> {
        let yaks_list = store.list_yaks()?;

        let mut yaks = HashMap::new();
        for yak in yaks_list {
            yaks.insert(
                yak.name,
                YakState {
                    id: yak.id,
                    state: yak.state,
                    context: yak.context,
                },
            );
        }

        Ok(Self {
            yaks,
            pending_events: Vec::new(),
        })
    }

    pub fn take_events(&mut self) -> Vec<YakEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn add_yak(
        &mut self,
        name: String,
        parent_id: Option<String>,
        context: Option<String>,
    ) -> Result<String> {
        // Validate parent exists
        if let Some(ref pid) = parent_id {
            if !self.yaks.values().any(|s| s.id == *pid) {
                anyhow::bail!("parent yak not found");
            }
        }

        let id = generate_slug(&name);

        // Construct internal path key from parent lookup
        let path_key = match &parent_id {
            Some(pid) => {
                let parent_path = self
                    .yaks
                    .iter()
                    .find(|(_, s)| s.id == *pid)
                    .map(|(k, _)| k.clone())
                    .unwrap(); // safe: validated above
                format!("{}/{}", parent_path, name)
            }
            None => name.clone(),
        };

        self.yaks.insert(
            path_key,
            YakState {
                id: id.clone(),
                state: "todo".to_string(),
                context: context.clone(),
            },
        );

        self.pending_events.push(YakEvent::Added(AddedEvent {
            name: name.clone(),
            id: id.clone(),
            parent_id,
        }));

        if let Some(content) = context {
            self.pending_events
                .push(YakEvent::ContextUpdated(ContextUpdatedEvent {
                    id: id.clone(),
                    content,
                }));
        }

        Ok(id)
    }

    fn ensure_ancestors_exist(&mut self, name: &str) {
        use crate::domain::get_ancestors;

        // Reverse so we create from root toward leaves (parent before child)
        let mut ancestors = get_ancestors(name);
        ancestors.reverse();

        for ancestor in ancestors {
            if !self.yaks.contains_key(&ancestor) {
                let leaf = ancestor.rsplit('/').next().unwrap_or(&ancestor);
                let id = generate_slug(leaf);
                let parent_id = self.parent_id_for(&ancestor);
                self.yaks.insert(
                    ancestor.clone(),
                    YakState {
                        id: id.clone(),
                        state: "todo".to_string(),
                        context: None,
                    },
                );
                self.pending_events.push(YakEvent::Added(AddedEvent {
                    name: leaf.to_string(),
                    id,
                    parent_id,
                }));
            }
        }
    }

    /// Look up the parent yak's id from the map, based on name-path hierarchy.
    fn parent_id_for(&self, name: &str) -> Option<String> {
        use crate::domain::hierarchy::get_parent;

        get_parent(name).and_then(|parent_name| self.yaks.get(&parent_name).map(|s| s.id.clone()))
    }

    pub fn update_state(&mut self, name: String, state: String) -> Result<()> {
        use crate::domain::validate_state;

        validate_state(&state).map_err(|e| anyhow::anyhow!(e))?;

        if !self.yaks.contains_key(&name) {
            anyhow::bail!("yak '{}' not found", name);
        }

        // Validate children if marking done
        if state == "done" {
            self.validate_children_complete(&name)?;
        }

        // Capture old state before updating
        let old_state = self.yaks.get(&name).unwrap().state.clone();
        let transitioning_from_todo = old_state == "todo" && state != "todo";
        let transitioning_from_done = old_state == "done" && state != "done";

        // Update this yak
        let yak = self.yaks.get_mut(&name).unwrap();
        yak.state = state.clone();
        let yak_id = yak.id.clone();
        self.pending_events
            .push(YakEvent::StateUpdated(StateUpdatedEvent {
                id: yak_id,
                state,
            }));

        // Propagate to ancestors if transitioning from todo
        if transitioning_from_todo {
            self.propagate_wip_to_ancestors(&name);
        }

        // Demote done ancestors if transitioning from done
        if transitioning_from_done {
            self.demote_done_ancestors_to_wip(&name);
        }

        Ok(())
    }

    fn validate_children_complete(&self, parent_name: &str) -> Result<()> {
        use crate::domain::find_children;

        let children = find_children(parent_name, &self.yaks);

        if !children.is_empty() {
            let incomplete = children
                .iter()
                .any(|name| self.yaks.get(name).unwrap().state != "done");

            if incomplete {
                anyhow::bail!(
                    "cannot mark '{}' as done - it has incomplete children",
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
                    let parent_id = parent.id.clone();
                    self.pending_events
                        .push(YakEvent::StateUpdated(StateUpdatedEvent {
                            id: parent_id,
                            state: "wip".to_string(),
                        }));
                }
            }
        }
    }

    fn demote_done_ancestors_to_wip(&mut self, child_name: &str) {
        use crate::domain::get_ancestors;

        for ancestor in get_ancestors(child_name) {
            if let Some(parent) = self.yaks.get_mut(&ancestor) {
                if parent.state == "done" {
                    parent.state = "wip".to_string();
                    let parent_id = parent.id.clone();
                    self.pending_events
                        .push(YakEvent::StateUpdated(StateUpdatedEvent {
                            id: parent_id,
                            state: "wip".to_string(),
                        }));
                }
            }
        }
    }

    pub fn update_context(&mut self, name: String, context: String) -> Result<()> {
        if !self.yaks.contains_key(&name) {
            anyhow::bail!("yak '{}' not found", name);
        }

        let yak = self.yaks.get_mut(&name).unwrap();
        yak.context = Some(context.clone());
        let yak_id = yak.id.clone();
        self.pending_events
            .push(YakEvent::ContextUpdated(ContextUpdatedEvent {
                id: yak_id,
                content: context,
            }));

        Ok(())
    }

    pub fn update_field(
        &mut self,
        name: String,
        field_name: String,
        content: String,
    ) -> Result<()> {
        if !self.yaks.contains_key(&name) {
            anyhow::bail!("yak '{}' not found", name);
        }

        let yak_id = self.yaks.get(&name).unwrap().id.clone();
        self.pending_events
            .push(YakEvent::FieldUpdated(FieldUpdatedEvent {
                id: yak_id,
                field_name,
                content,
            }));

        Ok(())
    }

    pub fn remove_yak(&mut self, name: String) -> Result<()> {
        use crate::domain::find_children;

        if !self.yaks.contains_key(&name) {
            anyhow::bail!("yak '{}' not found", name);
        }

        // Prevent removing yak with children (referential integrity)
        let children = find_children(&name, &self.yaks);
        if !children.is_empty() {
            anyhow::bail!(
                "Cannot remove '{}': it has {} child(ren). Remove children first.",
                name,
                children.len()
            );
        }

        let yak_id = self.yaks.get(&name).unwrap().id.clone();
        self.yaks.remove(&name);
        self.pending_events
            .push(YakEvent::Removed(RemovedEvent { id: yak_id }));

        Ok(())
    }

    pub fn prune(&mut self) -> Result<()> {
        use crate::domain::find_children;

        let done_leaves: Vec<String> = self
            .yaks
            .iter()
            .filter(|(name, state)| {
                state.state == "done" && find_children(name, &self.yaks).is_empty()
            })
            .map(|(name, _)| name.clone())
            .collect();

        for name in done_leaves {
            self.remove_yak(name)?;
        }

        Ok(())
    }

    pub fn move_yak(&mut self, old_name: String, new_name: String) -> Result<()> {
        use crate::domain::{find_children, validate_yak_name};

        if !self.yaks.contains_key(&old_name) {
            anyhow::bail!("yak '{}' not found", old_name);
        }

        if self.yaks.contains_key(&new_name) {
            anyhow::bail!("Yak '{}' already exists", new_name);
        }

        // Validate each segment of the new name
        for segment in new_name.split('/') {
            validate_yak_name(segment).map_err(|e| anyhow::anyhow!(e))?;
        }

        // MVP limitation: Fail if moving a yak with children
        let children = find_children(&old_name, &self.yaks);
        if !children.is_empty() {
            anyhow::bail!(
                "Cannot move '{}': it has {} child(ren). Moving with children is not yet supported.",
                old_name,
                children.len()
            );
        }

        // Ensure ancestors exist for new location
        self.ensure_ancestors_exist(&new_name);

        // Move the yak
        if let Some(yak_state) = self.yaks.remove(&old_name) {
            let yak_id = yak_state.id.clone();
            self.yaks.insert(new_name.clone(), yak_state);
            // Determine event type: rename vs move
            use crate::domain::hierarchy::get_parent;
            let old_parent = get_parent(&old_name);
            let new_parent_name = get_parent(&new_name);
            let old_leaf = old_name.rsplit('/').next().unwrap_or(&old_name);
            let new_leaf = new_name.rsplit('/').next().unwrap_or(&new_name);

            if old_parent == new_parent_name {
                // Same parent - just a rename
                self.pending_events.push(YakEvent::Renamed(RenamedEvent {
                    id: yak_id,
                    new_name: new_leaf.to_string(),
                }));
            } else {
                // Different parent - a move
                let new_parent_id = new_parent_name
                    .as_ref()
                    .and_then(|p| self.yaks.get(p))
                    .map(|s| s.id.clone());
                self.pending_events.push(YakEvent::Moved(MovedEvent {
                    id: yak_id.clone(),
                    new_parent: new_parent_id,
                }));
                // Also rename if leaf name changed
                if old_leaf != new_leaf {
                    self.pending_events.push(YakEvent::Renamed(RenamedEvent {
                        id: yak_id,
                        new_name: new_leaf.to_string(),
                    }));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a chain of nested yaks. Returns id of the last.
    fn add_nested_yaks(map: &mut YakMap, names: &[&str]) -> String {
        let mut parent_id: Option<String> = None;
        let mut last_id = String::new();
        for name in names {
            last_id = map
                .add_yak(name.to_string(), parent_id.clone(), None)
                .unwrap();
            parent_id = Some(last_id.clone());
        }
        last_id
    }

    #[test]
    fn test_new_yak_map_is_empty() {
        let map = YakMap::new();
        assert_eq!(map.yaks.len(), 0);
        assert_eq!(map.pending_events.len(), 0);
    }

    // Tests for from_store
    #[test]
    fn test_from_store_empty() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::Yak;
        use std::collections::HashMap;

        struct MockStore {
            yaks: HashMap<String, Yak>,
        }

        impl ReadYakStore for MockStore {
            fn get_yak(&self, name: &str) -> Result<Yak> {
                self.yaks
                    .get(name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Yak not found"))
            }

            fn list_yaks(&self) -> Result<Vec<Yak>> {
                Ok(self.yaks.values().cloned().collect())
            }

            fn yak_exists(&self, name: &str) -> bool {
                self.yaks.contains_key(name)
            }

            fn find_yak(&self, name: &str) -> Result<String> {
                if self.yaks.contains_key(name) {
                    Ok(name.to_string())
                } else {
                    anyhow::bail!("Yak not found")
                }
            }

            fn read_field(&self, _yak_name: &str, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not implemented")
            }
        }

        let store = MockStore {
            yaks: HashMap::new(),
        };
        let map = YakMap::from_store(&store).unwrap();

        assert_eq!(map.yaks.len(), 0);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_from_store_with_yaks() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::Yak;
        use std::collections::HashMap;

        struct MockStore {
            yaks: HashMap<String, Yak>,
        }

        impl ReadYakStore for MockStore {
            fn get_yak(&self, name: &str) -> Result<Yak> {
                self.yaks
                    .get(name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Yak not found"))
            }

            fn list_yaks(&self) -> Result<Vec<Yak>> {
                Ok(self.yaks.values().cloned().collect())
            }

            fn yak_exists(&self, name: &str) -> bool {
                self.yaks.contains_key(name)
            }

            fn find_yak(&self, name: &str) -> Result<String> {
                if self.yaks.contains_key(name) {
                    Ok(name.to_string())
                } else {
                    anyhow::bail!("Yak not found")
                }
            }

            fn read_field(&self, _yak_name: &str, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not implemented")
            }
        }

        let mut yaks = HashMap::new();
        yaks.insert(
            "test1".to_string(),
            Yak {
                id: "test1".to_string(),
                name: "test1".to_string(),
                state: "todo".to_string(),
                context: Some("context1".to_string()),
            },
        );
        yaks.insert(
            "test2".to_string(),
            Yak {
                id: "test2".to_string(),
                name: "test2".to_string(),
                state: "wip".to_string(),
                context: None,
            },
        );

        let store = MockStore { yaks };
        let map = YakMap::from_store(&store).unwrap();

        assert_eq!(map.yaks.len(), 2);
        assert_eq!(map.yaks.get("test1").unwrap().state, "todo");
        assert_eq!(
            map.yaks.get("test1").unwrap().context,
            Some("context1".to_string())
        );
        assert_eq!(map.yaks.get("test2").unwrap().state, "wip");
        assert_eq!(map.yaks.get("test2").unwrap().context, None);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_take_events_removes_events() {
        let mut map = YakMap::new();
        map.pending_events.push(YakEvent::Added(AddedEvent {
            name: "test".to_string(),
            id: String::new(),
            parent_id: None,
        }));

        let events = map.take_events();

        assert_eq!(events.len(), 1);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_add_yak_creates_yak_with_todo_state() {
        let mut map = YakMap::new();

        map.add_yak("test".to_string(), None, None).unwrap();

        assert!(map.yaks.contains_key("test"));
        assert_eq!(map.yaks.get("test").unwrap().state, "todo");
        assert_eq!(map.yaks.get("test").unwrap().context, None);
    }

    #[test]
    fn test_add_yak_generates_slug_id() {
        let mut map = YakMap::new();

        let id = map.add_yak("Make the tea".to_string(), None, None).unwrap();

        assert!(
            id.starts_with("make-the-tea-"),
            "Expected slug starting with 'make-the-tea-', got '{}'",
            id
        );
        assert_eq!(id.len(), "make-the-tea-".len() + 4);
    }

    #[test]
    fn test_add_yak_stores_id_in_yak_state() {
        let mut map = YakMap::new();

        let id = map.add_yak("test".to_string(), None, None).unwrap();

        assert_eq!(map.yaks.get("test").unwrap().id, id);
    }

    #[test]
    fn test_add_yak_with_context() {
        let mut map = YakMap::new();

        map.add_yak("test".to_string(), None, Some("context".to_string()))
            .unwrap();

        assert_eq!(
            map.yaks.get("test").unwrap().context,
            Some("context".to_string())
        );
    }

    #[test]
    fn test_add_yak_emits_added_event() {
        let mut map = YakMap::new();

        map.add_yak("test".to_string(), None, None).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Added(AddedEvent { name, .. }) => assert_eq!(name, "test"),
            _ => panic!("Expected Added event"),
        }
    }

    #[test]
    fn test_add_yak_with_context_emits_two_events() {
        let mut map = YakMap::new();

        map.add_yak("test".to_string(), None, Some("context".to_string()))
            .unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 2);
        match &events[0] {
            YakEvent::Added(AddedEvent { name, .. }) => assert_eq!(name, "test"),
            _ => panic!("Expected Added event first"),
        }
        match &events[1] {
            YakEvent::ContextUpdated(ContextUpdatedEvent { id, content }) => {
                assert!(!id.is_empty());
                assert_eq!(content, "context");
            }
            _ => panic!("Expected ContextUpdated event second"),
        }
    }

    #[test]
    fn test_add_yak_with_parent_id() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();
        assert!(map.yaks.contains_key("parent/child"));
    }

    #[test]
    fn test_add_yak_with_nonexistent_parent_fails() {
        let mut map = YakMap::new();
        let result = map.add_yak(
            "child".to_string(),
            Some("nonexistent-id".to_string()),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_yak_emits_leaf_name_in_event() {
        let mut map = YakMap::new();
        let pid = map.add_yak("parent".to_string(), None, None).unwrap();
        map.take_events();
        map.add_yak("child".to_string(), Some(pid.clone()), None)
            .unwrap();
        let events = map.take_events();
        match &events[0] {
            YakEvent::Added(e) => {
                assert_eq!(e.name, "child"); // leaf only!
                assert_eq!(e.parent_id, Some(pid));
            }
            _ => panic!("Expected Added event"),
        }
    }

    #[test]
    fn test_add_yak_child_preserves_parent_context() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent".to_string(), None, Some("context".to_string()))
            .unwrap();
        map.take_events();

        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();

        // Parent context should be preserved
        assert_eq!(
            map.yaks.get("parent").unwrap().context,
            Some("context".to_string())
        );

        // Only one Added event (for child)
        let events = map.take_events();
        assert_eq!(events.len(), 1);
    }

    // Tests for update_state
    #[test]
    fn test_update_state_changes_state() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();
        map.update_state("test".to_string(), "wip".to_string())
            .unwrap();
        assert_eq!(map.yaks.get("test").unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_validates_state() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None, None).unwrap();
        let result = map.update_state("test".to_string(), "invalid".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_update_state_prevents_marking_parent_done_with_incomplete_children() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();
        let result = map.update_state("parent".to_string(), "done".to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("incomplete children"));
    }

    #[test]
    fn test_update_state_allows_marking_parent_done_with_all_children_done() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();
        map.update_state("parent/child".to_string(), "done".to_string())
            .unwrap();
        let result = map.update_state("parent".to_string(), "done".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_state_propagates_to_parent_on_todo_transition() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();
        map.take_events();
        map.update_state("parent/child".to_string(), "wip".to_string())
            .unwrap();
        assert_eq!(map.yaks.get("parent").unwrap().state, "wip");
        assert_eq!(map.yaks.get("parent/child").unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_propagates_through_multiple_levels() {
        let mut map = YakMap::new();
        add_nested_yaks(&mut map, &["a", "b", "c"]);
        map.take_events();
        map.update_state("a/b/c".to_string(), "wip".to_string())
            .unwrap();
        assert_eq!(map.yaks.get("a").unwrap().state, "wip");
        assert_eq!(map.yaks.get("a/b").unwrap().state, "wip");
        assert_eq!(map.yaks.get("a/b/c").unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_only_propagates_on_todo_transition() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();
        map.update_state("parent/child".to_string(), "wip".to_string())
            .unwrap();
        map.take_events();
        map.update_state("parent/child".to_string(), "done".to_string())
            .unwrap();
        let events = map.take_events();
        assert_eq!(events.len(), 1); // Only child event
    }

    #[test]
    fn test_update_state_demotes_done_parent_when_child_leaves_done() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();
        map.update_state("parent/child".to_string(), "done".to_string())
            .unwrap();
        map.update_state("parent".to_string(), "done".to_string())
            .unwrap();
        map.take_events();
        map.update_state("parent/child".to_string(), "wip".to_string())
            .unwrap();
        assert_eq!(map.yaks.get("parent").unwrap().state, "wip");
        assert_eq!(map.yaks.get("parent/child").unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_demotes_through_multiple_levels() {
        let mut map = YakMap::new();
        add_nested_yaks(&mut map, &["a", "b", "c"]);
        map.update_state("a/b/c".to_string(), "done".to_string())
            .unwrap();
        map.update_state("a/b".to_string(), "done".to_string())
            .unwrap();
        map.update_state("a".to_string(), "done".to_string())
            .unwrap();
        map.take_events();
        map.update_state("a/b/c".to_string(), "wip".to_string())
            .unwrap();
        assert_eq!(map.yaks.get("a").unwrap().state, "wip");
        assert_eq!(map.yaks.get("a/b").unwrap().state, "wip");
        assert_eq!(map.yaks.get("a/b/c").unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_only_demotes_done_ancestors() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();
        map.update_state("parent/child".to_string(), "done".to_string())
            .unwrap();
        // parent is wip (auto-promoted), not done
        assert_eq!(map.yaks.get("parent").unwrap().state, "wip");
        map.take_events();
        map.update_state("parent/child".to_string(), "wip".to_string())
            .unwrap();
        // parent stays wip, not affected
        assert_eq!(map.yaks.get("parent").unwrap().state, "wip");
        let events = map.take_events();
        assert_eq!(events.len(), 1); // Only child event
    }

    // Tests for update_context
    #[test]
    fn test_update_context_updates_context() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();

        map.update_context("test".to_string(), "new context".to_string())
            .unwrap();

        assert_eq!(
            map.yaks.get("test").unwrap().context,
            Some("new context".to_string())
        );
    }

    #[test]
    fn test_update_context_emits_event() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();

        map.update_context("test".to_string(), "new context".to_string())
            .unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::ContextUpdated(ContextUpdatedEvent { id, content }) => {
                assert!(!id.is_empty());
                assert_eq!(content, "new context");
            }
            _ => panic!("Expected ContextUpdated event"),
        }
    }

    #[test]
    fn test_update_context_fails_for_nonexistent_yak() {
        let mut map = YakMap::new();
        let result = map.update_context("nonexistent".to_string(), "context".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // Tests for update_field
    #[test]
    fn test_update_field_emits_event() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();

        map.update_field(
            "test".to_string(),
            "notes".to_string(),
            "some content".to_string(),
        )
        .unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::FieldUpdated(FieldUpdatedEvent {
                id,
                field_name,
                content,
            }) => {
                assert!(!id.is_empty());
                assert_eq!(field_name, "notes");
                assert_eq!(content, "some content");
            }
            _ => panic!("Expected FieldUpdated event"),
        }
    }

    #[test]
    fn test_update_field_fails_for_nonexistent_yak() {
        let mut map = YakMap::new();
        let result = map.update_field(
            "nonexistent".to_string(),
            "notes".to_string(),
            "content".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // Tests for remove_yak
    #[test]
    fn test_remove_yak_removes_yak() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();

        map.remove_yak("test".to_string()).unwrap();

        assert!(!map.yaks.contains_key("test"));
    }

    #[test]
    fn test_remove_yak_emits_event() {
        let mut map = YakMap::new();
        map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();

        map.remove_yak("test".to_string()).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Removed(RemovedEvent { id }) => assert!(!id.is_empty()),
            _ => panic!("Expected Removed event"),
        }
    }

    #[test]
    fn test_remove_yak_fails_for_nonexistent_yak() {
        let mut map = YakMap::new();
        let result = map.remove_yak("nonexistent".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_remove_yak_fails_if_has_children() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();

        let result = map.remove_yak("parent".to_string());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("has"));
        assert!(err_msg.contains("child"));
    }

    // Tests for move_yak
    #[test]
    fn test_move_yak_moves_yak() {
        let mut map = YakMap::new();
        map.add_yak("old".to_string(), None, Some("context".to_string()))
            .unwrap();
        map.take_events();

        map.move_yak("old".to_string(), "new".to_string()).unwrap();

        assert!(!map.yaks.contains_key("old"));
        assert!(map.yaks.contains_key("new"));
        assert_eq!(
            map.yaks.get("new").unwrap().context,
            Some("context".to_string())
        );
    }

    #[test]
    fn test_move_yak_emits_renamed_event_for_same_level() {
        let mut map = YakMap::new();
        map.add_yak("old".to_string(), None, None).unwrap();
        map.take_events();

        map.move_yak("old".to_string(), "new".to_string()).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Renamed(RenamedEvent { id, new_name }) => {
                assert!(!id.is_empty());
                assert_eq!(new_name, "new");
            }
            _ => panic!("Expected Renamed event"),
        }
    }

    #[test]
    fn test_move_yak_creates_ancestors() {
        let mut map = YakMap::new();
        map.add_yak("old".to_string(), None, None).unwrap();
        map.take_events();

        map.move_yak("old".to_string(), "a/b/new".to_string())
            .unwrap();

        assert!(map.yaks.contains_key("a"));
        assert!(map.yaks.contains_key("a/b"));
        assert!(map.yaks.contains_key("a/b/new"));
    }

    #[test]
    fn test_move_yak_fails_for_nonexistent_yak() {
        let mut map = YakMap::new();
        let result = map.move_yak("nonexistent".to_string(), "new".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_move_yak_fails_if_destination_exists() {
        let mut map = YakMap::new();
        map.add_yak("old".to_string(), None, None).unwrap();
        map.add_yak("new".to_string(), None, None).unwrap();

        let result = map.move_yak("old".to_string(), "new".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_move_yak_fails_if_has_children() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();

        let result = map.move_yak("parent".to_string(), "newparent".to_string());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("has"));
        assert!(err_msg.contains("child"));
    }

    // Tests for prune
    #[test]
    fn test_prune_removes_done_leaf_yaks() {
        let mut map = YakMap::new();
        map.add_yak("done-yak".to_string(), None, None).unwrap();
        map.add_yak("todo-yak".to_string(), None, None).unwrap();
        map.update_state("done-yak".to_string(), "done".to_string())
            .unwrap();
        map.take_events();

        map.prune().unwrap();

        assert!(!map.yaks.contains_key("done-yak"));
        assert!(map.yaks.contains_key("todo-yak"));
    }

    #[test]
    fn test_prune_skips_done_parent_with_undone_children() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        map.add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();
        // Mark child done, then mark parent done
        map.update_state("parent/child".to_string(), "done".to_string())
            .unwrap();
        map.update_state("parent".to_string(), "done".to_string())
            .unwrap();
        map.take_events();

        map.prune().unwrap();

        // Child should be removed (done leaf)
        assert!(!map.yaks.contains_key("parent/child"));
        // Parent kept because it had children when we collected
        assert!(map.yaks.contains_key("parent"));
    }

    #[test]
    fn test_prune_emits_removed_events() {
        let mut map = YakMap::new();
        map.add_yak("done-yak".to_string(), None, None).unwrap();
        map.add_yak("todo-yak".to_string(), None, None).unwrap();
        map.update_state("done-yak".to_string(), "done".to_string())
            .unwrap();
        map.take_events();

        map.prune().unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Removed(RemovedEvent { id }) => {
                assert!(!id.is_empty())
            }
            _ => panic!("Expected Removed event"),
        }
    }
}
