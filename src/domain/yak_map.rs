use crate::domain::events::*;
use crate::domain::ports::ReadYakStore;
use crate::domain::slug::generate_id;
use crate::domain::YakEvent;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YakState {
    pub(crate) name: String,
    pub(crate) parent_id: Option<String>,
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

        // First pass: build name→id mapping
        let name_to_id: HashMap<String, String> = yaks_list
            .iter()
            .map(|yak| (yak.name.clone(), yak.id.clone()))
            .collect();

        // Second pass: populate with ID keys and derived parent_id
        let mut yaks = HashMap::new();
        for yak in &yaks_list {
            let leaf = yak.name.rsplit('/').next().unwrap_or(&yak.name);
            let parent_id = crate::domain::hierarchy::get_parent(&yak.name)
                .and_then(|parent_name| name_to_id.get(&parent_name))
                .cloned();
            yaks.insert(
                yak.id.clone(),
                YakState {
                    name: leaf.to_string(),
                    parent_id,
                    state: yak.state.clone(),
                    context: yak.context.clone(),
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

    /// Build the full display name for a yak by walking up the parent chain.
    fn build_display_name(&self, id: &str) -> String {
        let mut parts = Vec::new();
        let mut current_id = Some(id.to_string());

        while let Some(ref cid) = current_id {
            if let Some(state) = self.yaks.get(cid) {
                parts.push(state.name.clone());
                current_id = state.parent_id.clone();
            } else {
                break;
            }
        }

        parts.reverse();
        parts.join("/")
    }

    /// Resolve a name-path or ID to the yak's ID.
    fn resolve(&self, key: &str) -> Option<String> {
        // Try exact ID match
        if self.yaks.contains_key(key) {
            return Some(key.to_string());
        }

        // Try matching by display name
        for id in self.yaks.keys() {
            if self.build_display_name(id) == key {
                return Some(id.clone());
            }
        }

        None
    }

    /// Resolve a key, returning an error if not found.
    fn resolve_or_bail(&self, key: &str) -> Result<String> {
        self.resolve(key)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", key))
    }

    /// Find direct children of a yak by its ID.
    fn find_children_of(&self, parent_id: &str) -> Vec<String> {
        self.yaks
            .iter()
            .filter(|(_, state)| state.parent_id.as_deref() == Some(parent_id))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get ancestor IDs from immediate parent to root.
    fn get_ancestor_ids(&self, id: &str) -> Vec<String> {
        let mut ancestors = Vec::new();
        let mut current_id = self.yaks.get(id).and_then(|s| s.parent_id.clone());

        while let Some(pid) = current_id {
            ancestors.push(pid.clone());
            current_id = self.yaks.get(&pid).and_then(|s| s.parent_id.clone());
        }

        ancestors
    }

    pub fn add_yak(
        &mut self,
        name: String,
        parent_id: Option<String>,
        context: Option<String>,
    ) -> Result<String> {
        // Validate parent exists
        if let Some(ref pid) = parent_id {
            if !self.yaks.contains_key(pid) {
                anyhow::bail!("parent yak not found");
            }
        }

        let id = generate_id(&name);

        self.yaks.insert(
            id.clone(),
            YakState {
                name: name.clone(),
                parent_id: parent_id.clone(),
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

    fn ensure_ancestors_exist(&mut self, name_path: &str) {
        use crate::domain::hierarchy::get_ancestors;

        // Get ancestor name-paths in root-to-leaf order
        let mut ancestors = get_ancestors(name_path);
        ancestors.reverse();

        for ancestor_name in ancestors {
            // Check if this ancestor already exists (by display name)
            let exists = self.resolve(&ancestor_name).is_some();
            if !exists {
                let leaf = ancestor_name.rsplit('/').next().unwrap_or(&ancestor_name);
                let parent_name = crate::domain::hierarchy::get_parent(&ancestor_name);
                let parent_id = parent_name.and_then(|pn| self.resolve(&pn));
                let id = generate_id(leaf);
                self.yaks.insert(
                    id.clone(),
                    YakState {
                        name: leaf.to_string(),
                        parent_id: parent_id.clone(),
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

    pub fn update_state(&mut self, key: String, state: String) -> Result<()> {
        use crate::domain::validate_state;

        validate_state(&state).map_err(|e| anyhow::anyhow!(e))?;

        let id = self.resolve_or_bail(&key)?;

        // Validate children if marking done
        if state == "done" {
            self.validate_children_complete(&id)?;
        }

        // Capture old state before updating
        let old_state = self.yaks.get(&id).unwrap().state.clone();
        let transitioning_from_todo = old_state == "todo" && state != "todo";
        let transitioning_from_done = old_state == "done" && state != "done";

        // Update this yak
        let yak = self.yaks.get_mut(&id).unwrap();
        yak.state = state.clone();
        self.pending_events
            .push(YakEvent::StateUpdated(StateUpdatedEvent {
                id: id.clone(),
                state,
            }));

        // Propagate to ancestors if transitioning from todo
        if transitioning_from_todo {
            self.propagate_wip_to_ancestors(&id);
        }

        // Demote done ancestors if transitioning from done
        if transitioning_from_done {
            self.demote_done_ancestors_to_wip(&id);
        }

        Ok(())
    }

    fn validate_children_complete(&self, parent_id: &str) -> Result<()> {
        let children = self.find_children_of(parent_id);

        let incomplete = children
            .iter()
            .any(|cid| self.yaks.get(cid).unwrap().state != "done");

        if incomplete {
            let display = self.build_display_name(parent_id);
            anyhow::bail!(
                "cannot mark '{}' as done - it has incomplete children",
                display
            );
        }

        Ok(())
    }

    fn propagate_wip_to_ancestors(&mut self, id: &str) {
        for ancestor_id in self.get_ancestor_ids(id) {
            if let Some(parent) = self.yaks.get_mut(&ancestor_id) {
                if parent.state == "todo" {
                    parent.state = "wip".to_string();
                    self.pending_events
                        .push(YakEvent::StateUpdated(StateUpdatedEvent {
                            id: ancestor_id.clone(),
                            state: "wip".to_string(),
                        }));
                }
            }
        }
    }

    fn demote_done_ancestors_to_wip(&mut self, id: &str) {
        for ancestor_id in self.get_ancestor_ids(id) {
            if let Some(parent) = self.yaks.get_mut(&ancestor_id) {
                if parent.state == "done" {
                    parent.state = "wip".to_string();
                    self.pending_events
                        .push(YakEvent::StateUpdated(StateUpdatedEvent {
                            id: ancestor_id.clone(),
                            state: "wip".to_string(),
                        }));
                }
            }
        }
    }

    pub fn update_context(&mut self, key: String, context: String) -> Result<()> {
        let id = self.resolve_or_bail(&key)?;

        let yak = self.yaks.get_mut(&id).unwrap();
        yak.context = Some(context.clone());
        self.pending_events
            .push(YakEvent::ContextUpdated(ContextUpdatedEvent {
                id,
                content: context,
            }));

        Ok(())
    }

    pub fn update_field(&mut self, key: String, field_name: String, content: String) -> Result<()> {
        let id = self.resolve_or_bail(&key)?;

        self.pending_events
            .push(YakEvent::FieldUpdated(FieldUpdatedEvent {
                id,
                field_name,
                content,
            }));

        Ok(())
    }

    pub fn remove_yak(&mut self, key: String) -> Result<()> {
        let id = self.resolve_or_bail(&key)?;

        // Prevent removing yak with children (referential integrity)
        let children = self.find_children_of(&id);
        if !children.is_empty() {
            let display = self.build_display_name(&id);
            anyhow::bail!(
                "Cannot remove '{}': it has {} child(ren). Remove children first.",
                display,
                children.len()
            );
        }

        self.yaks.remove(&id);
        self.pending_events
            .push(YakEvent::Removed(RemovedEvent { id }));

        Ok(())
    }

    pub fn prune(&mut self) -> Result<()> {
        let done_leaves: Vec<String> = self
            .yaks
            .iter()
            .filter(|(id, state)| state.state == "done" && self.find_children_of(id).is_empty())
            .map(|(id, _)| id.clone())
            .collect();

        for id in done_leaves {
            self.yaks.remove(&id);
            self.pending_events
                .push(YakEvent::Removed(RemovedEvent { id }));
        }

        Ok(())
    }

    pub fn move_yak(&mut self, old_key: String, new_name: String) -> Result<()> {
        use crate::domain::validate_yak_name;

        let id = self.resolve_or_bail(&old_key)?;

        // Validate each segment of the new name
        for segment in new_name.split('/') {
            validate_yak_name(segment).map_err(|e| anyhow::anyhow!(e))?;
        }

        // MVP limitation: Fail if moving a yak with children
        let children = self.find_children_of(&id);
        if !children.is_empty() {
            let display = self.build_display_name(&id);
            anyhow::bail!(
                "Cannot move '{}': it has {} child(ren). Moving with children is not yet supported.",
                display,
                children.len()
            );
        }

        // Check if destination name already exists
        if self.resolve(&new_name).is_some() {
            anyhow::bail!("Yak '{}' already exists", new_name);
        }

        // Ensure ancestors exist for new location
        self.ensure_ancestors_exist(&new_name);

        // Determine old parent and leaf
        let old_parent_id = self.yaks.get(&id).unwrap().parent_id.clone();
        let old_leaf = self.yaks.get(&id).unwrap().name.clone();

        // Determine new parent and leaf from the name-path
        let new_leaf = new_name.rsplit('/').next().unwrap_or(&new_name);
        let new_parent_name = crate::domain::hierarchy::get_parent(&new_name);
        let new_parent_id = new_parent_name.and_then(|pn| self.resolve(&pn));

        // Update the yak in place
        let yak = self.yaks.get_mut(&id).unwrap();
        yak.name = new_leaf.to_string();
        yak.parent_id = new_parent_id.clone();

        // Emit events
        if old_parent_id == new_parent_id {
            // Same parent - just a rename
            self.pending_events.push(YakEvent::Renamed(RenamedEvent {
                id: id.clone(),
                new_name: new_leaf.to_string(),
            }));
        } else {
            // Different parent - a move
            self.pending_events.push(YakEvent::Moved(MovedEvent {
                id: id.clone(),
                new_parent: new_parent_id,
            }));
            // Also rename if leaf name changed
            if old_leaf != new_leaf {
                self.pending_events.push(YakEvent::Renamed(RenamedEvent {
                    id: id.clone(),
                    new_name: new_leaf.to_string(),
                }));
            }
        }

        Ok(())
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

    // Tests for from_store
    #[test]
    fn test_from_store_empty() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::Yak;

        struct MockStore;

        impl ReadYakStore for MockStore {
            fn get_yak(&self, _name: &str) -> Result<Yak> {
                anyhow::bail!("empty")
            }
            fn list_yaks(&self) -> Result<Vec<Yak>> {
                Ok(vec![])
            }
            fn yak_exists(&self, _name: &str) -> bool {
                false
            }
            fn find_yak(&self, _name: &str) -> Result<String> {
                anyhow::bail!("empty")
            }
            fn read_field(&self, _yak_name: &str, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not implemented")
            }
        }

        let store = MockStore;
        let map = YakMap::from_store(&store).unwrap();

        assert_eq!(map.yaks.len(), 0);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_from_store_with_yaks() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::Yak;

        struct MockStore {
            yaks: Vec<Yak>,
        }

        impl ReadYakStore for MockStore {
            fn get_yak(&self, name: &str) -> Result<Yak> {
                self.yaks
                    .iter()
                    .find(|y| y.name == name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Yak not found"))
            }

            fn list_yaks(&self) -> Result<Vec<Yak>> {
                Ok(self.yaks.clone())
            }

            fn yak_exists(&self, name: &str) -> bool {
                self.yaks.iter().any(|y| y.name == name)
            }

            fn find_yak(&self, name: &str) -> Result<String> {
                self.yaks
                    .iter()
                    .find(|y| y.name == name)
                    .map(|y| y.name.clone())
                    .ok_or_else(|| anyhow::anyhow!("Yak not found"))
            }

            fn read_field(&self, _yak_name: &str, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not implemented")
            }
        }

        let store = MockStore {
            yaks: vec![
                Yak {
                    id: "test1-aaaa".to_string(),
                    name: "test1".to_string(),
                    state: "todo".to_string(),
                    context: Some("context1".to_string()),
                },
                Yak {
                    id: "test2-bbbb".to_string(),
                    name: "test2".to_string(),
                    state: "wip".to_string(),
                    context: None,
                },
            ],
        };
        let map = YakMap::from_store(&store).unwrap();

        assert_eq!(map.yaks.len(), 2);
        assert_eq!(map.yaks.get("test1-aaaa").unwrap().state, "todo");
        assert_eq!(
            map.yaks.get("test1-aaaa").unwrap().context,
            Some("context1".to_string())
        );
        assert_eq!(map.yaks.get("test2-bbbb").unwrap().state, "wip");
        assert_eq!(map.yaks.get("test2-bbbb").unwrap().context, None);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_from_store_derives_parent_id() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::Yak;

        struct MockStore;

        impl ReadYakStore for MockStore {
            fn get_yak(&self, _name: &str) -> Result<Yak> {
                anyhow::bail!("Not needed")
            }

            fn list_yaks(&self) -> Result<Vec<Yak>> {
                Ok(vec![
                    Yak {
                        id: "parent-aaaa".to_string(),
                        name: "parent".to_string(),
                        state: "wip".to_string(),
                        context: None,
                    },
                    Yak {
                        id: "child-bbbb".to_string(),
                        name: "parent/child".to_string(),
                        state: "todo".to_string(),
                        context: None,
                    },
                ])
            }

            fn yak_exists(&self, _name: &str) -> bool {
                false
            }
            fn find_yak(&self, _name: &str) -> Result<String> {
                anyhow::bail!("Not needed")
            }
            fn read_field(&self, _yak_name: &str, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not needed")
            }
        }

        let map = YakMap::from_store(&MockStore).unwrap();
        let child = map.yaks.get("child-bbbb").unwrap();
        assert_eq!(child.name, "child");
        assert_eq!(child.parent_id, Some("parent-aaaa".to_string()));
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

        let id = map.add_yak("test".to_string(), None, None).unwrap();

        assert!(map.yaks.contains_key(&id));
        assert_eq!(map.yaks.get(&id).unwrap().state, "todo");
        assert_eq!(map.yaks.get(&id).unwrap().context, None);
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
    fn test_add_yak_stores_name_in_yak_state() {
        let mut map = YakMap::new();

        let id = map.add_yak("test".to_string(), None, None).unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().name, "test");
    }

    #[test]
    fn test_add_yak_with_context() {
        let mut map = YakMap::new();

        let id = map
            .add_yak("test".to_string(), None, Some("context".to_string()))
            .unwrap();

        assert_eq!(
            map.yaks.get(&id).unwrap().context,
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
        let child_id = map
            .add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();

        let child = map.yaks.get(&child_id).unwrap();
        assert_eq!(child.parent_id, Some(parent_id));
        assert_eq!(child.name, "child");
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

        map.add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();

        // Parent context should be preserved
        assert_eq!(
            map.yaks.get(&parent_id).unwrap().context,
            Some("context".to_string())
        );

        // Only one Added event (for child)
        let events = map.take_events();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_build_display_name_root() {
        let mut map = YakMap::new();
        let id = map.add_yak("test".to_string(), None, None).unwrap();
        assert_eq!(map.build_display_name(&id), "test");
    }

    #[test]
    fn test_build_display_name_nested() {
        let mut map = YakMap::new();
        let pid = map.add_yak("parent".to_string(), None, None).unwrap();
        let cid = map.add_yak("child".to_string(), Some(pid), None).unwrap();
        assert_eq!(map.build_display_name(&cid), "parent/child");
    }

    #[test]
    fn test_resolve_by_id() {
        let mut map = YakMap::new();
        let id = map.add_yak("test".to_string(), None, None).unwrap();
        assert_eq!(map.resolve(&id), Some(id.clone()));
    }

    #[test]
    fn test_resolve_by_display_name() {
        let mut map = YakMap::new();
        let pid = map.add_yak("parent".to_string(), None, None).unwrap();
        let cid = map.add_yak("child".to_string(), Some(pid), None).unwrap();
        assert_eq!(map.resolve("parent/child"), Some(cid));
    }

    // Tests for update_state
    #[test]
    fn test_update_state_changes_state() {
        let mut map = YakMap::new();
        let id = map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();
        map.update_state(id.clone(), "wip".to_string()).unwrap();
        assert_eq!(map.yaks.get(&id).unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_by_display_name() {
        let mut map = YakMap::new();
        let id = map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();
        map.update_state("test".to_string(), "wip".to_string())
            .unwrap();
        assert_eq!(map.yaks.get(&id).unwrap().state, "wip");
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
        map.add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();
        let result = map.update_state(parent_id, "done".to_string());
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
        let child_id = map
            .add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();
        map.update_state(child_id, "done".to_string()).unwrap();
        let result = map.update_state(parent_id, "done".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_state_propagates_to_parent_on_todo_transition() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        let child_id = map
            .add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();
        map.take_events();
        map.update_state(child_id.clone(), "wip".to_string())
            .unwrap();
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "wip");
        assert_eq!(map.yaks.get(&child_id).unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_propagates_through_multiple_levels() {
        let mut map = YakMap::new();
        let a_id = map.add_yak("a".to_string(), None, None).unwrap();
        let b_id = map
            .add_yak("b".to_string(), Some(a_id.clone()), None)
            .unwrap();
        let c_id = map
            .add_yak("c".to_string(), Some(b_id.clone()), None)
            .unwrap();
        map.take_events();
        map.update_state(c_id.clone(), "wip".to_string()).unwrap();
        assert_eq!(map.yaks.get(&a_id).unwrap().state, "wip");
        assert_eq!(map.yaks.get(&b_id).unwrap().state, "wip");
        assert_eq!(map.yaks.get(&c_id).unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_only_propagates_on_todo_transition() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        let child_id = map
            .add_yak("child".to_string(), Some(parent_id), None)
            .unwrap();
        map.update_state(child_id.clone(), "wip".to_string())
            .unwrap();
        map.take_events();
        map.update_state(child_id, "done".to_string()).unwrap();
        let events = map.take_events();
        assert_eq!(events.len(), 1); // Only child event
    }

    #[test]
    fn test_update_state_demotes_done_parent_when_child_leaves_done() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        let child_id = map
            .add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();
        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();
        map.update_state(parent_id.clone(), "done".to_string())
            .unwrap();
        map.take_events();
        map.update_state(child_id.clone(), "wip".to_string())
            .unwrap();
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "wip");
        assert_eq!(map.yaks.get(&child_id).unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_demotes_through_multiple_levels() {
        let mut map = YakMap::new();
        let a_id = map.add_yak("a".to_string(), None, None).unwrap();
        let b_id = map
            .add_yak("b".to_string(), Some(a_id.clone()), None)
            .unwrap();
        let c_id = map
            .add_yak("c".to_string(), Some(b_id.clone()), None)
            .unwrap();
        map.update_state(c_id.clone(), "done".to_string()).unwrap();
        map.update_state(b_id.clone(), "done".to_string()).unwrap();
        map.update_state(a_id.clone(), "done".to_string()).unwrap();
        map.take_events();
        map.update_state(c_id.clone(), "wip".to_string()).unwrap();
        assert_eq!(map.yaks.get(&a_id).unwrap().state, "wip");
        assert_eq!(map.yaks.get(&b_id).unwrap().state, "wip");
        assert_eq!(map.yaks.get(&c_id).unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_only_demotes_done_ancestors() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        let child_id = map
            .add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();
        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();
        // parent is wip (auto-promoted), not done
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "wip");
        map.take_events();
        map.update_state(child_id, "wip".to_string()).unwrap();
        // parent stays wip, not affected
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "wip");
        let events = map.take_events();
        assert_eq!(events.len(), 1); // Only child event
    }

    // Tests for update_context
    #[test]
    fn test_update_context_updates_context() {
        let mut map = YakMap::new();
        let id = map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();

        map.update_context(id.clone(), "new context".to_string())
            .unwrap();

        assert_eq!(
            map.yaks.get(&id).unwrap().context,
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
        let id = map.add_yak("test".to_string(), None, None).unwrap();
        map.take_events();

        map.remove_yak(id.clone()).unwrap();

        assert!(!map.yaks.contains_key(&id));
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
        map.add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();

        let result = map.remove_yak(parent_id);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("has"));
        assert!(err_msg.contains("child"));
    }

    // Tests for move_yak
    #[test]
    fn test_move_yak_renames() {
        let mut map = YakMap::new();
        let id = map
            .add_yak("old".to_string(), None, Some("context".to_string()))
            .unwrap();
        map.take_events();

        map.move_yak(id.clone(), "new".to_string()).unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().name, "new");
        assert_eq!(
            map.yaks.get(&id).unwrap().context,
            Some("context".to_string())
        );
    }

    #[test]
    fn test_move_yak_emits_renamed_event_for_same_level() {
        let mut map = YakMap::new();
        let id = map.add_yak("old".to_string(), None, None).unwrap();
        map.take_events();

        map.move_yak(id.clone(), "new".to_string()).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Renamed(RenamedEvent {
                id: event_id,
                new_name,
            }) => {
                assert_eq!(event_id, &id);
                assert_eq!(new_name, "new");
            }
            _ => panic!("Expected Renamed event"),
        }
    }

    #[test]
    fn test_move_yak_creates_ancestors() {
        let mut map = YakMap::new();
        let id = map.add_yak("old".to_string(), None, None).unwrap();
        map.take_events();

        map.move_yak(id.clone(), "a/b/new".to_string()).unwrap();

        // The yak should now have parent chain a -> b -> new
        assert_eq!(map.yaks.get(&id).unwrap().name, "new");
        assert!(map.resolve("a").is_some());
        assert!(map.resolve("a/b").is_some());
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
        map.add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();

        let result = map.move_yak(parent_id, "newparent".to_string());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("has"));
        assert!(err_msg.contains("child"));
    }

    // Tests for prune
    #[test]
    fn test_prune_removes_done_leaf_yaks() {
        let mut map = YakMap::new();
        let done_id = map.add_yak("done-yak".to_string(), None, None).unwrap();
        let todo_id = map.add_yak("todo-yak".to_string(), None, None).unwrap();
        map.update_state(done_id.clone(), "done".to_string())
            .unwrap();
        map.take_events();

        map.prune().unwrap();

        assert!(!map.yaks.contains_key(&done_id));
        assert!(map.yaks.contains_key(&todo_id));
    }

    #[test]
    fn test_prune_skips_done_parent_with_undone_children() {
        let mut map = YakMap::new();
        let parent_id = map.add_yak("parent".to_string(), None, None).unwrap();
        let child_id = map
            .add_yak("child".to_string(), Some(parent_id.clone()), None)
            .unwrap();
        // Mark child done, then mark parent done
        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();
        map.update_state(parent_id.clone(), "done".to_string())
            .unwrap();
        map.take_events();

        map.prune().unwrap();

        // Child should be removed (done leaf)
        assert!(!map.yaks.contains_key(&child_id));
        // Parent kept because it had children when we collected
        assert!(map.yaks.contains_key(&parent_id));
    }

    #[test]
    fn test_prune_emits_removed_events() {
        let mut map = YakMap::new();
        let done_id = map.add_yak("done-yak".to_string(), None, None).unwrap();
        map.add_yak("todo-yak".to_string(), None, None).unwrap();
        map.update_state(done_id, "done".to_string()).unwrap();
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
