use crate::domain::event_metadata::EventMetadata;
use crate::domain::events::*;
use crate::domain::ports::ReadYakStore;
use crate::domain::slug::{generate_id, slugify, Name, YakId};
use crate::domain::YakEvent;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YakState {
    pub(crate) name: Name,
    pub(crate) parent_id: Option<YakId>,
    pub(crate) state: String,
    pub(crate) context: Option<String>,
}

pub struct YakMap {
    yaks: HashMap<YakId, YakState>,
    pending_events: Vec<YakEvent>,
    metadata: EventMetadata,
}

impl YakMap {
    #[cfg(test)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            yaks: HashMap::new(),
            pending_events: Vec::new(),
            metadata: EventMetadata::default_legacy(),
        }
    }

    pub fn with_metadata(metadata: EventMetadata) -> Self {
        Self {
            yaks: HashMap::new(),
            pending_events: Vec::new(),
            metadata,
        }
    }

    pub fn from_store(store: &dyn ReadYakStore, metadata: EventMetadata) -> Result<Self> {
        let yaks_list = store.list_yaks()?;

        let mut yaks = HashMap::new();
        for yak in &yaks_list {
            // Stores now return leaf names directly (no slash-splitting needed).
            // Use parent_id from YakView struct directly.
            yaks.insert(
                yak.id.clone(),
                YakState {
                    name: yak.name.clone(),
                    parent_id: yak.parent_id.clone(),
                    state: yak.state.clone(),
                    context: yak.context.clone(),
                },
            );
        }

        Ok(Self {
            yaks,
            pending_events: Vec::new(),
            metadata,
        })
    }

    pub fn take_events(&mut self) -> Vec<YakEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Build the full display name for a yak by walking up the parent chain.
    fn build_display_name(&self, id: &YakId) -> String {
        let mut parts = Vec::new();
        let mut current_id = Some(id.clone());

        while let Some(ref cid) = current_id {
            if let Some(state) = self.yaks.get(cid) {
                parts.push(state.name.to_string());
                current_id = state.parent_id.clone();
            } else {
                break;
            }
        }

        parts.reverse();
        parts.join("/")
    }

    /// Verify a YakId exists in the map, returning an error if not found.
    fn ensure_exists(&self, id: &YakId) -> Result<()> {
        if self.yaks.contains_key(id) {
            Ok(())
        } else {
            anyhow::bail!("yak '{}' not found", id)
        }
    }

    /// Find direct children of a yak by its ID.
    fn find_children_of(&self, parent_id: &YakId) -> Vec<YakId> {
        self.yaks
            .iter()
            .filter(|(_, state)| state.parent_id.as_ref() == Some(parent_id))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get ancestor IDs from immediate parent to root.
    fn get_ancestor_ids(&self, id: &YakId) -> Vec<YakId> {
        let mut ancestors = Vec::new();
        let mut current_id = self.yaks.get(id).and_then(|s| s.parent_id.clone());

        while let Some(pid) = current_id {
            ancestors.push(pid.clone());
            current_id = self.yaks.get(&pid).and_then(|s| s.parent_id.clone());
        }

        ancestors
    }

    /// Check that no sibling under the same parent has the same slug.
    /// `self_id` is used to exclude the yak being renamed from the check.
    fn check_sibling_slug_uniqueness(
        &self,
        name: &str,
        parent_id: &Option<YakId>,
        self_id: Option<&YakId>,
    ) -> Result<()> {
        let new_slug = slugify(name);

        for (id, state) in &self.yaks {
            // Skip the yak itself (for rename case)
            if let Some(sid) = self_id {
                if id == sid {
                    continue;
                }
            }

            // Only check siblings (same parent)
            if &state.parent_id != parent_id {
                continue;
            }

            let sibling_slug = slugify(state.name.as_str());
            if sibling_slug.as_str() == new_slug.as_str() {
                let msg = match parent_id {
                    Some(pid) => {
                        let parent_display = self.build_display_name(pid);
                        format!(
                            "A yak named \"{}\" already exists \
                             under \"{}\" with the same slug \
                             \"{}\". Try a more distinct name.",
                            state.name, parent_display, new_slug
                        )
                    }
                    None => {
                        format!(
                            "A yak named \"{}\" already exists \
                             with the same slug \"{}\". \
                             Try a more distinct name.",
                            state.name, new_slug
                        )
                    }
                };
                anyhow::bail!(msg);
            }
        }

        Ok(())
    }

    pub fn add_yak(
        &mut self,
        name: impl Into<Name>,
        parent_id: Option<YakId>,
        context: Option<String>,
        state: Option<String>,
        explicit_id: Option<YakId>,
        fields: Vec<(String, String)>,
    ) -> Result<YakId> {
        use crate::domain::validate_state;

        let name = name.into();

        // Validate state if provided
        let initial_state = if let Some(ref s) = state {
            validate_state(s).map_err(|e| anyhow::anyhow!(e))?;
            s.clone()
        } else {
            "todo".to_string()
        };

        // Validate parent exists
        if let Some(ref pid) = parent_id {
            if !self.yaks.contains_key(pid) {
                anyhow::bail!("parent yak not found");
            }
        }

        // Check slug uniqueness among siblings
        self.check_sibling_slug_uniqueness(name.as_str(), &parent_id, None)?;

        let id = explicit_id.unwrap_or_else(|| generate_id(name.as_str(), parent_id.as_ref()));

        self.yaks.insert(
            id.clone(),
            YakState {
                name: name.clone(),
                parent_id: parent_id.clone(),
                state: initial_state.clone(),
                context: context.clone(),
            },
        );

        self.pending_events.push(YakEvent::Added(
            AddedEvent {
                name: name.clone(),
                id: id.clone(),
                parent_id,
            },
            self.metadata.clone(),
        ));

        if let Some(content) = context {
            self.pending_events.push(YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: id.clone(),
                    field_name: ".context.md".to_string(),
                    content,
                },
                self.metadata.clone(),
            ));
        }

        if initial_state != "todo" {
            self.pending_events.push(YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: id.clone(),
                    field_name: ".state".to_string(),
                    content: initial_state,
                },
                self.metadata.clone(),
            ));
        }

        for (field_name, content) in fields {
            self.pending_events.push(YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: id.clone(),
                    field_name,
                    content,
                },
                self.metadata.clone(),
            ));
        }

        // Demote done ancestors to todo when a new child is added
        self.demote_done_ancestors_to_todo(&id);

        Ok(id)
    }

    pub fn update_state(&mut self, id: YakId, state: String) -> Result<()> {
        use crate::domain::validate_state;

        validate_state(&state).map_err(|e| anyhow::anyhow!(e))?;

        self.ensure_exists(&id)?;

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
        self.pending_events.push(YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id: id.clone(),
                field_name: ".state".to_string(),
                content: state,
            },
            self.metadata.clone(),
        ));

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

    fn validate_children_complete(&self, parent_id: &YakId) -> Result<()> {
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

    fn propagate_wip_to_ancestors(&mut self, id: &YakId) {
        for ancestor_id in self.get_ancestor_ids(id) {
            if let Some(parent) = self.yaks.get_mut(&ancestor_id) {
                if parent.state == "todo" {
                    parent.state = "wip".to_string();
                    self.pending_events.push(YakEvent::FieldUpdated(
                        FieldUpdatedEvent {
                            id: ancestor_id.clone(),
                            field_name: ".state".to_string(),
                            content: "wip".to_string(),
                        },
                        self.metadata.clone(),
                    ));
                }
            }
        }
    }

    fn demote_done_ancestors_to_todo(&mut self, id: &YakId) {
        for ancestor_id in self.get_ancestor_ids(id) {
            if let Some(parent) = self.yaks.get_mut(&ancestor_id) {
                if parent.state == "done" {
                    parent.state = "todo".to_string();
                    self.pending_events.push(YakEvent::FieldUpdated(
                        FieldUpdatedEvent {
                            id: ancestor_id.clone(),
                            field_name: ".state".to_string(),
                            content: "todo".to_string(),
                        },
                        self.metadata.clone(),
                    ));
                }
            }
        }
    }

    fn demote_done_ancestors_to_wip(&mut self, id: &YakId) {
        for ancestor_id in self.get_ancestor_ids(id) {
            if let Some(parent) = self.yaks.get_mut(&ancestor_id) {
                if parent.state == "done" {
                    parent.state = "wip".to_string();
                    self.pending_events.push(YakEvent::FieldUpdated(
                        FieldUpdatedEvent {
                            id: ancestor_id.clone(),
                            field_name: ".state".to_string(),
                            content: "wip".to_string(),
                        },
                        self.metadata.clone(),
                    ));
                }
            }
        }
    }

    pub fn update_context(&mut self, id: YakId, context: String) -> Result<()> {
        self.ensure_exists(&id)?;

        let yak = self.yaks.get_mut(&id).unwrap();
        yak.context = Some(context.clone());
        self.pending_events.push(YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id,
                field_name: ".context.md".to_string(),
                content: context,
            },
            self.metadata.clone(),
        ));

        Ok(())
    }

    pub fn update_field(&mut self, id: YakId, field_name: String, content: String) -> Result<()> {
        self.ensure_exists(&id)?;

        self.pending_events.push(YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id,
                field_name,
                content,
            },
            self.metadata.clone(),
        ));

        Ok(())
    }

    pub fn remove_yak(&mut self, id: YakId) -> Result<()> {
        self.ensure_exists(&id)?;

        // Prevent removing yak with children (referential integrity)
        let children = self.find_children_of(&id);
        if !children.is_empty() {
            let display = self.build_display_name(&id);
            anyhow::bail!(
                "Cannot remove '{}': it has {} child(ren). Use --recursive to remove it and all its descendants.",
                display,
                children.len()
            );
        }

        self.yaks.remove(&id);
        self.pending_events.push(YakEvent::Removed(
            RemovedEvent { id },
            self.metadata.clone(),
        ));

        Ok(())
    }

    pub fn prune(&mut self) -> Result<()> {
        loop {
            let done_leaves: Vec<YakId> = self
                .yaks
                .iter()
                .filter(|(id, state)| state.state == "done" && self.find_children_of(id).is_empty())
                .map(|(id, _)| id.clone())
                .collect();

            if done_leaves.is_empty() {
                break;
            }

            for id in done_leaves {
                self.yaks.remove(&id);
                self.pending_events.push(YakEvent::Removed(
                    RemovedEvent { id },
                    self.metadata.clone(),
                ));
            }
        }

        Ok(())
    }

    pub fn rename_yak(&mut self, id: YakId, new_name: String) -> Result<()> {
        use crate::domain::validate_yak_name;

        self.ensure_exists(&id)?;

        validate_yak_name(&new_name).map_err(|e| anyhow::anyhow!(e))?;

        // Get the current parent_id (rename does NOT change parent)
        let parent_id = self.yaks.get(&id).unwrap().parent_id.clone();

        // Check slug uniqueness among siblings (excluding self)
        self.check_sibling_slug_uniqueness(&new_name, &parent_id, Some(&id))?;

        // Update the name in place
        let yak = self.yaks.get_mut(&id).unwrap();
        yak.name = Name::from(new_name.as_str());

        // Emit FieldUpdated event for name change
        self.pending_events.push(YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id,
                field_name: ".name".to_string(),
                content: new_name.to_string(),
            },
            self.metadata.clone(),
        ));

        Ok(())
    }

    /// Move a yak to a new parent (or to root if new_parent_id is None).
    /// The yak keeps its current name.
    pub fn move_yak_to(&mut self, id: YakId, new_parent_id: Option<YakId>) -> Result<()> {
        self.ensure_exists(&id)?;

        // Validate new parent exists
        if let Some(ref pid) = new_parent_id {
            self.ensure_exists(pid)?;
        }

        // Prevent moving a yak under itself
        if let Some(ref pid) = new_parent_id {
            if id == *pid {
                anyhow::bail!(
                    "Cannot move '{}' under itself",
                    self.yaks.get(&id).unwrap().name
                );
            }
        }

        // Prevent moving a yak under its own descendant (cycle detection)
        if let Some(ref pid) = new_parent_id {
            let mut current = Some(pid.clone());
            while let Some(ref cid) = current {
                if *cid == id {
                    let target_name = &self.yaks.get(pid).unwrap().name;
                    anyhow::bail!(
                        "Cannot move '{}' under its own descendant '{}'",
                        self.yaks.get(&id).unwrap().name,
                        target_name
                    );
                }
                current = self.yaks.get(cid).and_then(|y| y.parent_id.clone());
            }
        }

        let old_parent_id = self.yaks.get(&id).unwrap().parent_id.clone();

        // No-op if already at the desired position
        if old_parent_id == new_parent_id {
            return Ok(());
        }

        // Check slug uniqueness among siblings at the destination
        let name = self.yaks.get(&id).unwrap().name.as_str().to_string();
        self.check_sibling_slug_uniqueness(&name, &new_parent_id, Some(&id))?;

        // Update the yak's parent
        let yak = self.yaks.get_mut(&id).unwrap();
        yak.parent_id = new_parent_id.clone();

        // Emit Moved event
        self.pending_events.push(YakEvent::Moved(
            MovedEvent {
                id,
                new_parent: new_parent_id,
            },
            self.metadata.clone(),
        ));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::slug::Name;

    // ==================================================================
    // Tests for slug uniqueness among siblings
    // ==================================================================

    #[test]
    fn test_add_yak_rejects_colliding_slug_at_root() {
        let mut map = YakMap::new();
        map.add_yak("Make the tea", None, None, None, None, vec![])
            .unwrap();

        let result = map.add_yak("make-the-tea", None, None, None, None, vec![]);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Make the tea"),
            "Error should mention existing yak name, got: {}",
            err
        );
        assert!(
            err.contains("make-the-tea"),
            "Error should mention the slug, got: {}",
            err
        );
        assert!(
            err.contains("Try a more distinct name"),
            "Error should suggest a fix, got: {}",
            err
        );
    }

    #[test]
    fn test_add_yak_rejects_extra_spaces_colliding_slug_at_root() {
        let mut map = YakMap::new();
        map.add_yak("Make the tea", None, None, None, None, vec![])
            .unwrap();

        // "Make  the  tea" slugifies to "make-the-tea" (same slug)
        let result = map.add_yak("Make  the  tea", None, None, None, None, vec![]);

        assert!(result.is_err());
    }

    #[test]
    fn test_add_yak_allows_different_slug_at_root() {
        let mut map = YakMap::new();
        map.add_yak("Make the tea", None, None, None, None, vec![])
            .unwrap();

        // "Make the_tea" slugifies to "make-thetea" (different slug)
        let result = map.add_yak("Make the_tea", None, None, None, None, vec![]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_add_yak_rejects_colliding_slug_under_same_parent() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("Backend fixes", None, None, None, None, vec![])
            .unwrap();
        map.add_yak(
            "Fix the bug",
            Some(parent_id.clone()),
            None,
            None,
            None,
            vec![],
        )
        .unwrap();

        let result = map.add_yak(
            "fix-the-bug",
            Some(parent_id.clone()),
            None,
            None,
            None,
            vec![],
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Fix the bug"),
            "Error should mention existing yak, got: {}",
            err
        );
        assert!(
            err.contains("Backend fixes"),
            "Error should mention parent name, got: {}",
            err
        );
    }

    #[test]
    fn test_add_yak_allows_same_slug_under_different_parent() {
        let mut map = YakMap::new();
        map.add_yak("Make the tea", None, None, None, None, vec![])
            .unwrap();
        let parent_id = map
            .add_yak("Backend fixes", None, None, None, None, vec![])
            .unwrap();

        let result = map.add_yak("Make the tea", Some(parent_id), None, None, None, vec![]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_add_yak_allows_same_slug_under_different_parents() {
        let mut map = YakMap::new();
        let backend = map
            .add_yak("Backend fixes", None, None, None, None, vec![])
            .unwrap();
        let frontend = map
            .add_yak("Frontend fixes", None, None, None, None, vec![])
            .unwrap();
        map.add_yak("Fix the bug", Some(backend), None, None, None, vec![])
            .unwrap();

        let result = map.add_yak("Fix the bug", Some(frontend), None, None, None, vec![]);

        assert!(result.is_ok());
    }

    #[test]
    fn test_rename_rejects_colliding_slug_with_sibling() {
        let mut map = YakMap::new();
        map.add_yak("Make the tea", None, None, None, None, vec![])
            .unwrap();
        let fix_id = map
            .add_yak("Fix the bug", None, None, None, None, vec![])
            .unwrap();

        let result = map.rename_yak(fix_id, "Make THE Tea".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("make-the-tea"),
            "Error should mention slug, got: {}",
            err
        );
    }

    #[test]
    fn test_rename_allows_same_slug_for_self() {
        let mut map = YakMap::new();
        let id = map
            .add_yak("Make the tea", None, None, None, None, vec![])
            .unwrap();

        // Rename to different capitalisation (same slug)
        let result = map.rename_yak(id, "Make The Tea".to_string());

        assert!(result.is_ok());
    }

    #[test]
    fn test_move_to_rejects_colliding_slug_at_destination() {
        let mut map = YakMap::new();
        map.add_yak("Fix the bug", None, None, None, None, vec![])
            .unwrap();
        let backend = map
            .add_yak("Backend fixes", None, None, None, None, vec![])
            .unwrap();
        let nested_fix = map
            .add_yak("Fix the bug", Some(backend), None, None, None, vec![])
            .unwrap();

        // Move nested "Fix the bug" to root - collides with root "Fix the bug"

        let result = map.move_yak_to(nested_fix, None);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("fix-the-bug"),
            "Error should mention slug, got: {}",
            err
        );
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
        use crate::domain::YakView;

        struct MockStore;

        impl ReadYakStore for MockStore {
            fn get_yak(&self, _id: &YakId) -> Result<YakView> {
                anyhow::bail!("empty")
            }
            fn list_yaks(&self) -> Result<Vec<YakView>> {
                Ok(vec![])
            }
            fn fuzzy_find_yak_id(&self, _query: &str) -> Result<YakId> {
                anyhow::bail!("empty")
            }
            fn read_field(&self, _id: &YakId, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not implemented")
            }
        }

        let store = MockStore;
        let map = YakMap::from_store(&store, EventMetadata::default_legacy()).unwrap();

        assert_eq!(map.yaks.len(), 0);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_from_store_with_yaks() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::YakView;

        struct MockStore {
            yaks: Vec<YakView>,
        }

        impl ReadYakStore for MockStore {
            fn get_yak(&self, id: &YakId) -> Result<YakView> {
                self.yaks
                    .iter()
                    .find(|y| y.id == *id)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Yak not found"))
            }

            fn list_yaks(&self) -> Result<Vec<YakView>> {
                Ok(self.yaks.clone())
            }

            fn fuzzy_find_yak_id(&self, name: &str) -> Result<YakId> {
                self.yaks
                    .iter()
                    .find(|y| y.name.as_str() == name)
                    .map(|y| y.id.clone())
                    .ok_or_else(|| anyhow::anyhow!("Yak not found"))
            }

            fn read_field(&self, _id: &YakId, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not implemented")
            }
        }

        use crate::domain::event_metadata::{Author, Timestamp};
        let store = MockStore {
            yaks: vec![
                YakView {
                    id: YakId::from("test1-aaaa"),
                    name: Name::from("test1"),
                    parent_id: None,
                    state: "todo".to_string(),
                    context: Some("context1".to_string()),
                    fields: std::collections::HashMap::new(),
                    tags: vec![],
                    children: vec![],
                    created_by: Author::unknown(),
                    created_at: Timestamp::zero(),
                },
                YakView {
                    id: YakId::from("test2-bbbb"),
                    name: Name::from("test2"),
                    parent_id: None,
                    state: "wip".to_string(),
                    context: None,
                    fields: std::collections::HashMap::new(),
                    tags: vec![],
                    children: vec![],
                    created_by: Author::unknown(),
                    created_at: Timestamp::zero(),
                },
            ],
        };
        let map = YakMap::from_store(&store, EventMetadata::default_legacy()).unwrap();

        assert_eq!(map.yaks.len(), 2);
        assert_eq!(
            map.yaks.get(&YakId::from("test1-aaaa")).unwrap().state,
            "todo"
        );
        assert_eq!(
            map.yaks.get(&YakId::from("test1-aaaa")).unwrap().context,
            Some("context1".to_string())
        );
        assert_eq!(
            map.yaks.get(&YakId::from("test2-bbbb")).unwrap().state,
            "wip"
        );
        assert_eq!(
            map.yaks.get(&YakId::from("test2-bbbb")).unwrap().context,
            None
        );
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_from_store_uses_parent_id_and_leaf_name() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::YakView;

        struct MockStore;

        impl ReadYakStore for MockStore {
            fn get_yak(&self, _id: &YakId) -> Result<YakView> {
                anyhow::bail!("Not needed")
            }

            fn list_yaks(&self) -> Result<Vec<YakView>> {
                use crate::domain::event_metadata::{Author, Timestamp};
                Ok(vec![
                    YakView {
                        id: YakId::from("parent-aaaa"),
                        name: Name::from("parent"),
                        parent_id: None,
                        state: "wip".to_string(),
                        context: None,
                        fields: std::collections::HashMap::new(),
                        tags: vec![],
                        children: vec![],
                        created_by: Author::unknown(),
                        created_at: Timestamp::zero(),
                    },
                    YakView {
                        // Stores now return leaf names with explicit parent_id
                        id: YakId::from("child-bbbb"),
                        name: Name::from("child"),
                        parent_id: Some(YakId::from("parent-aaaa")),
                        state: "todo".to_string(),
                        context: None,
                        fields: std::collections::HashMap::new(),
                        tags: vec![],
                        children: vec![],
                        created_by: Author::unknown(),
                        created_at: Timestamp::zero(),
                    },
                ])
            }

            fn fuzzy_find_yak_id(&self, _query: &str) -> Result<YakId> {
                anyhow::bail!("Not needed")
            }
            fn read_field(&self, _id: &YakId, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not needed")
            }
        }

        let map = YakMap::from_store(&MockStore, EventMetadata::default_legacy()).unwrap();
        let child = map.yaks.get(&YakId::from("child-bbbb")).unwrap();
        assert_eq!(child.name, Name::from("child"));
        assert_eq!(child.parent_id, Some(YakId::from("parent-aaaa")));
    }

    #[test]
    fn test_from_store_uses_parent_id_from_yak() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::YakView;

        struct MockStore;

        impl ReadYakStore for MockStore {
            fn get_yak(&self, _id: &YakId) -> Result<YakView> {
                anyhow::bail!("Not needed")
            }

            fn list_yaks(&self) -> Result<Vec<YakView>> {
                use crate::domain::event_metadata::{Author, Timestamp};
                Ok(vec![
                    YakView {
                        id: YakId::from("parent-aaaa"),
                        name: Name::from("parent"),
                        parent_id: None,
                        state: "wip".to_string(),
                        context: None,
                        fields: std::collections::HashMap::new(),
                        tags: vec![],
                        children: vec![],
                        created_by: Author::unknown(),
                        created_at: Timestamp::zero(),
                    },
                    YakView {
                        id: YakId::from("child-bbbb"),
                        // Leaf-only name: no slash to derive parent from
                        name: Name::from("child"),
                        // parent_id explicitly set by store
                        parent_id: Some(YakId::from("parent-aaaa")),
                        state: "todo".to_string(),
                        context: None,
                        fields: std::collections::HashMap::new(),
                        tags: vec![],
                        children: vec![],
                        created_by: Author::unknown(),
                        created_at: Timestamp::zero(),
                    },
                ])
            }

            fn fuzzy_find_yak_id(&self, _query: &str) -> Result<YakId> {
                anyhow::bail!("Not needed")
            }
            fn read_field(&self, _id: &YakId, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not needed")
            }
        }

        let map = YakMap::from_store(&MockStore, EventMetadata::default_legacy()).unwrap();
        let child = map.yaks.get(&YakId::from("child-bbbb")).unwrap();
        assert_eq!(child.name, Name::from("child"));
        assert_eq!(
            child.parent_id,
            Some(YakId::from("parent-aaaa")),
            "from_store should use parent_id from YakView struct"
        );
    }

    #[test]
    fn test_take_events_removes_events() {
        let mut map = YakMap::new();
        map.pending_events.push(YakEvent::Added(
            AddedEvent {
                name: Name::from("test"),
                id: YakId::from(""),
                parent_id: None,
            },
            EventMetadata::default_legacy(),
        ));

        let events = map.take_events();

        assert_eq!(events.len(), 1);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_add_yak_creates_yak_with_todo_state() {
        let mut map = YakMap::new();

        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();

        assert!(map.yaks.contains_key(&id));
        assert_eq!(map.yaks.get(&id).unwrap().state, "todo");
        assert_eq!(map.yaks.get(&id).unwrap().context, None);
    }

    #[test]
    fn test_add_yak_generates_slug_id() {
        let mut map = YakMap::new();

        let id = map
            .add_yak("Make the tea", None, None, None, None, vec![])
            .unwrap();

        assert!(
            id.as_str().starts_with("make-the-tea-"),
            "Expected slug starting with 'make-the-tea-', got '{}'",
            id
        );
        assert_eq!(id.as_str().len(), "make-the-tea-".len() + 4);
    }

    #[test]
    fn test_add_yak_stores_name_in_yak_state() {
        let mut map = YakMap::new();

        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().name, Name::from("test"));
    }

    #[test]
    fn test_add_yak_with_context() {
        let mut map = YakMap::new();

        let id = map
            .add_yak(
                "test",
                None,
                Some("context".to_string()),
                None,
                None,
                vec![],
            )
            .unwrap();

        assert_eq!(
            map.yaks.get(&id).unwrap().context,
            Some("context".to_string())
        );
    }

    #[test]
    fn test_add_yak_emits_added_event() {
        let mut map = YakMap::new();

        map.add_yak("test", None, None, None, None, vec![]).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Added(AddedEvent { name, .. }, _) => {
                assert_eq!(name, &Name::from("test"))
            }
            _ => panic!("Expected Added event"),
        }
    }

    #[test]
    fn test_add_yak_with_context_emits_two_events() {
        let mut map = YakMap::new();

        map.add_yak(
            "test",
            None,
            Some("context".to_string()),
            None,
            None,
            vec![],
        )
        .unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 2);
        match &events[0] {
            YakEvent::Added(AddedEvent { name, .. }, _) => {
                assert_eq!(name, &Name::from("test"))
            }
            _ => panic!("Expected Added event first"),
        }
        match &events[1] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id,
                    field_name,
                    content,
                },
                _,
            ) => {
                assert!(!id.as_str().is_empty());
                assert_eq!(field_name, ".context.md");
                assert_eq!(content, "context");
            }
            _ => panic!("Expected FieldUpdated event second"),
        }
    }

    #[test]
    fn test_add_yak_with_parent_id() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();

        let child = map.yaks.get(&child_id).unwrap();
        assert_eq!(child.parent_id, Some(parent_id));
        assert_eq!(child.name, Name::from("child"));
    }

    #[test]
    fn test_add_yak_with_nonexistent_parent_fails() {
        let mut map = YakMap::new();
        let result = map.add_yak(
            "child",
            Some(YakId::from("nonexistent-id")),
            None,
            None,
            None,
            vec![],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_add_yak_emits_leaf_name_in_event() {
        let mut map = YakMap::new();
        let pid = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        map.take_events();
        map.add_yak("child", Some(pid.clone()), None, None, None, vec![])
            .unwrap();
        let events = map.take_events();
        match &events[0] {
            YakEvent::Added(e, _) => {
                assert_eq!(e.name, Name::from("child")); // leaf only!
                assert_eq!(e.parent_id, Some(pid));
            }
            _ => panic!("Expected Added event"),
        }
    }

    #[test]
    fn test_add_yak_child_preserves_parent_context() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak(
                "parent",
                None,
                Some("context".to_string()),
                None,
                None,
                vec![],
            )
            .unwrap();
        map.take_events();

        map.add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
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
    fn test_add_yak_demotes_done_parent_to_todo() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        map.update_state(parent_id.clone(), "done".to_string())
            .unwrap();
        map.take_events();

        map.add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();

        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "todo");
        let events = map.take_events();
        // Added + FieldUpdated(state=todo for parent)
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_add_yak_demotes_done_ancestors_through_multiple_levels() {
        let mut map = YakMap::new();
        let a_id = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let b_id = map
            .add_yak("b", Some(a_id.clone()), None, None, None, vec![])
            .unwrap();
        map.update_state(b_id.clone(), "done".to_string()).unwrap();
        map.update_state(a_id.clone(), "done".to_string()).unwrap();
        map.take_events();

        map.add_yak("c", Some(b_id.clone()), None, None, None, vec![])
            .unwrap();

        assert_eq!(map.yaks.get(&a_id).unwrap().state, "todo");
        assert_eq!(map.yaks.get(&b_id).unwrap().state, "todo");
    }

    #[test]
    fn test_add_yak_does_not_demote_non_done_parent() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        // Parent is "todo" (default)
        map.take_events();

        map.add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();

        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "todo");
        let events = map.take_events();
        // Only Added event (no state change for parent)
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_build_display_name_root() {
        let mut map = YakMap::new();
        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();
        assert_eq!(map.build_display_name(&id), "test");
    }

    #[test]
    fn test_build_display_name_nested() {
        let mut map = YakMap::new();
        let pid = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let cid = map
            .add_yak("child", Some(pid), None, None, None, vec![])
            .unwrap();
        assert_eq!(map.build_display_name(&cid), "parent/child");
    }

    // Tests for update_state
    #[test]
    fn test_update_state_changes_state() {
        let mut map = YakMap::new();
        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();
        map.take_events();
        map.update_state(id.clone(), "wip".to_string()).unwrap();
        assert_eq!(map.yaks.get(&id).unwrap().state, "wip");
    }

    #[test]
    fn test_update_state_validates_state() {
        let mut map = YakMap::new();
        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();
        let result = map.update_state(id, "invalid".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_update_state_prevents_marking_parent_done_with_incomplete_children() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        map.add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
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
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        map.update_state(child_id, "done".to_string()).unwrap();
        let result = map.update_state(parent_id, "done".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_state_propagates_to_parent_on_todo_transition() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
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
        let a_id = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let b_id = map
            .add_yak("b", Some(a_id.clone()), None, None, None, vec![])
            .unwrap();
        let c_id = map
            .add_yak("c", Some(b_id.clone()), None, None, None, vec![])
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
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id), None, None, None, vec![])
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
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
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
        let a_id = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let b_id = map
            .add_yak("b", Some(a_id.clone()), None, None, None, vec![])
            .unwrap();
        let c_id = map
            .add_yak("c", Some(b_id.clone()), None, None, None, vec![])
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
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();
        // parent is wip (auto-promoted), not done
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "wip");
        map.take_events();
        map.update_state(child_id.clone(), "wip".to_string())
            .unwrap();
        // parent stays wip, not affected
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "wip");
        let events = map.take_events();
        assert_eq!(events.len(), 1); // Only child event
    }

    // Tests for update_context
    #[test]
    fn test_update_context_updates_context() {
        let mut map = YakMap::new();
        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();
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
        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();
        map.take_events();

        map.update_context(id, "new context".to_string()).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id,
                    field_name,
                    content,
                },
                _,
            ) => {
                assert!(!id.as_str().is_empty());
                assert_eq!(field_name, ".context.md");
                assert_eq!(content, "new context");
            }
            _ => panic!("Expected FieldUpdated event"),
        }
    }

    #[test]
    fn test_update_context_fails_for_nonexistent_yak() {
        let mut map = YakMap::new();
        let result = map.update_context(YakId::from("nonexistent"), "context".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // Tests for update_field
    #[test]
    fn test_update_field_emits_event() {
        let mut map = YakMap::new();
        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();
        map.take_events();

        map.update_field(id, "notes".to_string(), "some content".to_string())
            .unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id,
                    field_name,
                    content,
                },
                _,
            ) => {
                assert!(!id.as_str().is_empty());
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
            YakId::from("nonexistent"),
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
        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();
        map.take_events();

        map.remove_yak(id.clone()).unwrap();

        assert!(!map.yaks.contains_key(&id));
    }

    #[test]
    fn test_remove_yak_emits_event() {
        let mut map = YakMap::new();
        let id = map.add_yak("test", None, None, None, None, vec![]).unwrap();
        map.take_events();

        map.remove_yak(id).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Removed(RemovedEvent { id }, _) => {
                assert!(!id.as_str().is_empty())
            }
            _ => panic!("Expected Removed event"),
        }
    }

    #[test]
    fn test_remove_yak_fails_for_nonexistent_yak() {
        let mut map = YakMap::new();
        let result = map.remove_yak(YakId::from("nonexistent"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_remove_yak_fails_if_has_children() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        map.add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();

        let result = map.remove_yak(parent_id);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("has"));
        assert!(err_msg.contains("child"));
    }

    // Tests for rename_yak
    #[test]
    fn test_rename_preserves_context() {
        let mut map = YakMap::new();
        let id = map
            .add_yak("old", None, Some("context".to_string()), None, None, vec![])
            .unwrap();
        map.take_events();

        map.rename_yak(id.clone(), "new".to_string()).unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().name, Name::from("new"));
        assert_eq!(
            map.yaks.get(&id).unwrap().context,
            Some("context".to_string())
        );
    }

    #[test]
    fn test_rename_emits_renamed_event() {
        let mut map = YakMap::new();
        let id = map.add_yak("old", None, None, None, None, vec![]).unwrap();
        map.take_events();

        map.rename_yak(id.clone(), "new".to_string()).unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: event_id,
                    field_name,
                    content,
                },
                _,
            ) => {
                assert_eq!(event_id, &id);
                assert_eq!(field_name, ".name");
                assert_eq!(content, "new");
            }
            _ => panic!("Expected FieldUpdated event"),
        }
    }

    // Tests for move_yak_to
    #[test]
    fn test_move_yak_with_children_moves_subtree() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        let dest_id = map.add_yak("dest", None, None, None, None, vec![]).unwrap();

        map.move_yak_to(parent_id.clone(), Some(dest_id.clone()))
            .unwrap();

        // parent is now under dest
        assert_eq!(map.yaks.get(&parent_id).unwrap().parent_id, Some(dest_id));
        // child is still under parent
        assert_eq!(map.yaks.get(&child_id).unwrap().parent_id, Some(parent_id));
    }

    #[test]
    fn test_move_yak_under_itself_returns_error() {
        let mut map = YakMap::new();
        let id = map.add_yak("yak", None, None, None, None, vec![]).unwrap();
        let result = map.move_yak_to(id.clone(), Some(id));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("under itself"),
            "Expected 'under itself' in: {}",
            err
        );
    }

    #[test]
    fn test_move_yak_under_own_descendant_returns_error() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        let result = map.move_yak_to(parent_id, Some(child_id));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("descendant"),
            "Expected 'descendant' in: {}",
            err
        );
    }

    #[test]
    fn test_move_yak_under_deep_descendant_returns_error() {
        let mut map = YakMap::new();
        let a = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let b = map
            .add_yak("b", Some(a.clone()), None, None, None, vec![])
            .unwrap();
        let c = map
            .add_yak("c", Some(b.clone()), None, None, None, vec![])
            .unwrap();
        let result = map.move_yak_to(a, Some(c));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("descendant"),
            "Expected 'descendant' in: {}",
            err
        );
    }

    // Tests for prune
    #[test]
    fn test_prune_removes_done_leaf_yaks() {
        let mut map = YakMap::new();
        let done_id = map
            .add_yak("done-yak", None, None, None, None, vec![])
            .unwrap();
        let todo_id = map
            .add_yak("todo-yak", None, None, None, None, vec![])
            .unwrap();
        map.update_state(done_id.clone(), "done".to_string())
            .unwrap();
        map.take_events();

        map.prune().unwrap();

        assert!(!map.yaks.contains_key(&done_id));
        assert!(map.yaks.contains_key(&todo_id));
    }

    #[test]
    fn test_prune_cascades_through_done_hierarchy() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        // Mark child done, then mark parent done
        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();
        map.update_state(parent_id.clone(), "done".to_string())
            .unwrap();
        map.take_events();

        map.prune().unwrap();

        // Both removed: child first (done leaf), then parent
        // becomes a done leaf and is removed on the next pass.
        assert!(!map.yaks.contains_key(&child_id));
        assert!(!map.yaks.contains_key(&parent_id));
    }

    #[test]
    fn test_prune_emits_removed_events() {
        let mut map = YakMap::new();
        let done_id = map
            .add_yak("done-yak", None, None, None, None, vec![])
            .unwrap();
        map.add_yak("todo-yak", None, None, None, None, vec![])
            .unwrap();
        map.update_state(done_id, "done".to_string()).unwrap();
        map.take_events();

        map.prune().unwrap();
        let events = map.take_events();

        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::Removed(RemovedEvent { id }, _) => {
                assert!(!id.as_str().is_empty())
            }
            _ => panic!("Expected Removed event"),
        }
    }

    // Tests for enriched add_yak parameters
    #[test]
    fn test_add_yak_with_initial_state() {
        let mut map = YakMap::new();

        let id = map
            .add_yak("test", None, None, Some("wip".to_string()), None, vec![])
            .unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().state, "wip");

        let events = map.take_events();
        // Should emit Added + FieldUpdated(state=wip)
        assert_eq!(events.len(), 2);
        match &events[1] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: event_id,
                    field_name,
                    content,
                },
                _,
            ) => {
                assert_eq!(event_id, &id);
                assert_eq!(field_name, ".state");
                assert_eq!(content, "wip");
            }
            _ => panic!("Expected FieldUpdated event for state"),
        }
    }

    #[test]
    fn test_add_yak_with_invalid_state_fails() {
        let mut map = YakMap::new();

        let result = map.add_yak(
            "test",
            None,
            None,
            Some("invalid".to_string()),
            None,
            vec![],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_add_yak_with_explicit_id() {
        let mut map = YakMap::new();

        let id = map
            .add_yak(
                "test",
                None,
                None,
                None,
                Some(YakId::from("custom-id")),
                vec![],
            )
            .unwrap();

        assert_eq!(id, YakId::from("custom-id"));
        assert!(map.yaks.contains_key(&YakId::from("custom-id")));
    }

    #[test]
    fn test_add_yak_with_fields() {
        let mut map = YakMap::new();

        let id = map
            .add_yak(
                "test",
                None,
                None,
                None,
                None,
                vec![
                    ("plan".to_string(), "my plan".to_string()),
                    ("notes".to_string(), "some notes".to_string()),
                ],
            )
            .unwrap();

        let events = map.take_events();
        // Added + 2 FieldUpdated events for custom fields
        assert_eq!(events.len(), 3);
        match &events[1] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: event_id,
                    field_name,
                    content,
                },
                _,
            ) => {
                assert_eq!(event_id, &id);
                assert_eq!(field_name, "plan");
                assert_eq!(content, "my plan");
            }
            _ => panic!("Expected FieldUpdated event for plan"),
        }
        match &events[2] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: event_id,
                    field_name,
                    content,
                },
                _,
            ) => {
                assert_eq!(event_id, &id);
                assert_eq!(field_name, "notes");
                assert_eq!(content, "some notes");
            }
            _ => panic!("Expected FieldUpdated event for notes"),
        }
    }

    #[test]
    fn test_add_yak_stamps_provided_metadata() {
        use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

        let metadata = EventMetadata::new(
            Author {
                name: "Matt".to_string(),
                email: "matt@example.com".to_string(),
            },
            Timestamp(1708300800),
        );
        let mut map = YakMap::with_metadata(metadata.clone());
        map.add_yak("test", None, None, None, None, vec![]).unwrap();
        let events = map.take_events();

        assert_eq!(events[0].metadata(), &metadata);
    }

    // Tests for state propagation transition conditions (lines 273-274)
    // These catch mutants where && is replaced with || in:
    //   transitioning_from_todo = old_state == "todo" && state != "todo"
    //   transitioning_from_done = old_state == "done" && state != "done"

    #[test]
    fn test_wip_to_done_does_not_promote_todo_parent() {
        // Catch mutant: `old_state == "todo" || state != "todo"`
        // would fire propagate_wip_to_ancestors on wip->done,
        // incorrectly promoting a todo parent to wip.
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        // Add child with initial state "wip" so parent stays "todo"
        let child_id = map
            .add_yak(
                "child",
                Some(parent_id.clone()),
                None,
                Some("wip".to_string()),
                None,
                vec![],
            )
            .unwrap();
        map.take_events();

        // Transition child from wip->done (not from todo)
        map.update_state(child_id, "done".to_string()).unwrap();

        // Parent should remain "todo" - propagation should NOT fire
        assert_eq!(
            map.yaks.get(&parent_id).unwrap().state,
            "todo",
            "Parent state should not be changed when child transitions wip->done"
        );
        let events = map.take_events();
        assert_eq!(
            events.len(),
            1,
            "Only one event (child state change) should be emitted"
        );
    }

    #[test]
    fn test_todo_to_wip_does_not_demote_done_parent() {
        // Catch mutant: `old_state == "done" || state != "done"`
        // would fire demote_done_ancestors_to_wip on todo->wip,
        // incorrectly demoting a done parent to wip.
        //
        // We use a parent that is "done" with no children, then
        // directly set a child's state (without adding under done parent,
        // since add now demotes done parents).
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        // Add child while parent is still todo
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        // Mark child done so parent can be done
        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();
        map.update_state(parent_id.clone(), "done".to_string())
            .unwrap();
        // Now reopen the child and re-done it to get parent back to done
        map.update_state(child_id.clone(), "todo".to_string())
            .unwrap();
        // Parent is now "wip" (demoted from done)
        // Mark child done again and parent done again
        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();
        map.update_state(parent_id.clone(), "done".to_string())
            .unwrap();
        // Now reopen child to todo
        map.update_state(child_id.clone(), "todo".to_string())
            .unwrap();
        // Parent is now "wip" (demoted from done by demote_done_ancestors_to_wip)
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, "wip");
        map.take_events();

        // Transition child from todo->wip (not from done)
        map.update_state(child_id, "wip".to_string()).unwrap();

        // Parent should remain "wip" - demote should NOT fire (parent isn't done)
        assert_eq!(
            map.yaks.get(&parent_id).unwrap().state,
            "wip",
            "Parent state should not be changed when child transitions todo->wip and parent is wip"
        );
        let events = map.take_events();
        assert_eq!(
            events.len(),
            1,
            "Only one event (child state change) should be emitted"
        );
    }

    #[test]
    fn test_add_yak_with_all_options() {
        let mut map = YakMap::new();

        let id = map
            .add_yak(
                "test",
                None,
                Some("context".to_string()),
                Some("wip".to_string()),
                Some(YakId::from("my-id")),
                vec![("plan".to_string(), "the plan".to_string())],
            )
            .unwrap();

        assert_eq!(id, YakId::from("my-id"));
        assert_eq!(map.yaks.get(&id).unwrap().state, "wip");
        assert_eq!(
            map.yaks.get(&id).unwrap().context,
            Some("context".to_string())
        );

        let events = map.take_events();
        // Added + FieldUpdated(context.md) + FieldUpdated(state) + FieldUpdated(plan)
        assert_eq!(events.len(), 4);
        match &events[0] {
            YakEvent::Added(
                AddedEvent {
                    name, id: event_id, ..
                },
                _,
            ) => {
                assert_eq!(name, &Name::from("test"));
                assert_eq!(event_id, &YakId::from("my-id"));
            }
            _ => panic!("Expected Added event first"),
        }
        match &events[1] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    field_name,
                    content,
                    ..
                },
                _,
            ) => {
                assert_eq!(field_name, ".context.md");
                assert_eq!(content, "context");
            }
            _ => panic!("Expected FieldUpdated for context.md second"),
        }
        match &events[2] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    field_name,
                    content,
                    ..
                },
                _,
            ) => {
                assert_eq!(field_name, ".state");
                assert_eq!(content, "wip");
            }
            _ => panic!("Expected FieldUpdated for state third"),
        }
        match &events[3] {
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    field_name,
                    content,
                    ..
                },
                _,
            ) => {
                assert_eq!(field_name, "plan");
                assert_eq!(content, "the plan");
            }
            _ => panic!("Expected FieldUpdated for plan fourth"),
        }
    }
}
