use crate::domain::event_metadata::EventMetadata;
use crate::domain::events::*;
use crate::domain::ports::ReadYakStore;
use crate::domain::slug::{generate_id, slugify, Name, YakId};
use crate::domain::yak_state::YakState;
use crate::domain::{ManualBlockerSnapshot, Yak, YakBlockerSnapshot, YakEvent, YakMapSnapshot};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

pub const MIGRATED_BLOCKED_REASON: &str = "Migrated from blocked state";
pub const MANUAL_BLOCKER_FIELD: &str = ".manual-blocker";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockingDependency {
    pub source: BlockerSource,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerPath {
    pub yaks: Vec<YakId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddBlockerOutcome {
    Added,
    Updated,
    AlreadyExplicit,
    AlreadyImpliedByHierarchy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveBlockerOutcome {
    Removed,
    NotPresent,
}

pub struct YakMap {
    yaks: HashMap<YakId, Yak>,
    blockers: HashMap<YakId, HashMap<BlockerSource, Option<String>>>,
    pending_events: Vec<YakEvent>,
    metadata: EventMetadata,
}

impl YakMap {
    #[cfg(test)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            yaks: HashMap::new(),
            blockers: HashMap::new(),
            pending_events: Vec::new(),
            metadata: EventMetadata::default_legacy(),
        }
    }

    pub fn with_metadata(metadata: EventMetadata) -> Self {
        Self {
            yaks: HashMap::new(),
            blockers: HashMap::new(),
            pending_events: Vec::new(),
            metadata,
        }
    }

    pub fn from_store(store: &dyn ReadYakStore, metadata: EventMetadata) -> Result<Self> {
        let yaks_list = store.list_yaks()?;

        let mut yaks = HashMap::new();
        let mut blockers: HashMap<YakId, HashMap<BlockerSource, Option<String>>> = HashMap::new();
        for mut yak in yaks_list {
            if yak.state == YakState::Blocked {
                yak.state = YakState::Todo;
                blockers.entry(yak.id.clone()).or_default().insert(
                    BlockerSource::Manual,
                    Some(MIGRATED_BLOCKED_REASON.to_string()),
                );
            }
            if let Some(reason) = yak.fields.remove(MANUAL_BLOCKER_FIELD) {
                if !reason.trim().is_empty() {
                    blockers
                        .entry(yak.id.clone())
                        .or_default()
                        .insert(BlockerSource::Manual, Some(reason));
                }
            }
            yaks.insert(yak.id.clone(), yak);
        }

        Ok(Self {
            yaks,
            blockers,
            pending_events: Vec::new(),
            metadata,
        })
    }

    /// Build YakMap by replaying events from the event store
    pub fn from_events(events: Vec<YakEvent>, metadata: EventMetadata) -> Result<Self> {
        let mut yak_map = Self {
            yaks: HashMap::new(),
            blockers: HashMap::new(),
            pending_events: Vec::new(),
            metadata,
        };

        for event in events {
            yak_map.apply(event)?;
        }

        Ok(yak_map)
    }

    /// Apply a single event to update the YakMap state
    fn apply(&mut self, event: YakEvent) -> Result<()> {
        match event {
            YakEvent::Added(added, metadata) => self.apply_added(added, metadata),
            YakEvent::Removed(removed, _) => self.apply_removed(removed),
            YakEvent::Moved(moved, _) => self.apply_moved(moved),
            YakEvent::FieldUpdated(field_updated, _) => self.apply_field_updated(field_updated),
            YakEvent::BlockerAdded(e, _) => {
                self.apply_blocker_added_or_updated(e.target, e.blocker)
            }
            YakEvent::BlockerUpdated(e, _) => {
                self.apply_blocker_added_or_updated(e.target, e.blocker)
            }
            YakEvent::BlockerRemoved(e, _) => self.apply_blocker_removed(e),
            YakEvent::ManualBlockerAdded(e, _) => self.apply_blocker_added_or_updated(
                e.target,
                Blocker {
                    source: BlockerSource::Manual,
                    reason: Some(e.reason),
                },
            ),
            YakEvent::ManualBlockerUpdated(e, _) => self.apply_blocker_added_or_updated(
                e.target,
                Blocker {
                    source: BlockerSource::Manual,
                    reason: Some(e.reason),
                },
            ),
            YakEvent::ManualBlockerRemoved(e, _) => self.apply_blocker_removed(e.into()),
            YakEvent::Compacted(snapshot, _) | YakEvent::Migrated(snapshot, _) => {
                self.apply_compacted(snapshot)
            }
        }
        Ok(())
    }

    fn apply_added(&mut self, added: AddedEvent, metadata: EventMetadata) {
        self.yaks.insert(
            added.id.clone(),
            Yak {
                id: added.id,
                name: added.name,
                parent_id: added.parent_id,
                state: YakState::Todo,
                context: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: metadata.author,
                created_at: metadata.timestamp,
            },
        );
    }

    fn apply_removed(&mut self, removed: RemovedEvent) {
        self.yaks.remove(&removed.id);
        self.blockers.remove(&removed.id);
        for blockers in self.blockers.values_mut() {
            blockers.remove(&BlockerSource::Yak(removed.id.clone()));
        }
    }

    fn apply_moved(&mut self, moved: MovedEvent) {
        if let Some(yak) = self.yaks.get_mut(&moved.id) {
            yak.parent_id = moved.new_parent;
        }
    }

    fn apply_field_updated(&mut self, field_updated: FieldUpdatedEvent) {
        if field_updated.field_name == ".state" {
            if field_updated.content == "blocked" {
                if let Some(yak) = self.yaks.get_mut(&field_updated.id) {
                    yak.state = YakState::Todo;
                    self.blockers
                        .entry(field_updated.id.clone())
                        .or_default()
                        .insert(
                            BlockerSource::Manual,
                            Some(MIGRATED_BLOCKED_REASON.to_string()),
                        );
                }
                return;
            }
            if self
                .blockers
                .get(&field_updated.id)
                .and_then(|blockers| blockers.get(&BlockerSource::Manual))
                .is_some_and(|reason| reason.as_deref() == Some(MIGRATED_BLOCKED_REASON))
            {
                if let Some(blockers) = self.blockers.get_mut(&field_updated.id) {
                    blockers.remove(&BlockerSource::Manual);
                }
            }
        }
        if let Some(yak) = self.yaks.get_mut(&field_updated.id) {
            Self::apply_field_update_to_yak(yak, field_updated);
        }
    }

    fn apply_field_update_to_yak(yak: &mut Yak, field_updated: FieldUpdatedEvent) {
        match field_updated.field_name.as_str() {
            ".state" => {
                if !field_updated.content.is_empty() {
                    yak.state = field_updated.content.parse().unwrap_or(YakState::Todo);
                }
            }
            ".context.md" => {
                yak.context = if field_updated.content.is_empty() {
                    None
                } else {
                    Some(field_updated.content)
                };
            }
            ".name" => {
                yak.name = Name::from(field_updated.content.as_str());
            }
            ".tags" => {
                yak.tags = field_updated
                    .content
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(String::from)
                    .collect();
            }
            _ => Self::apply_custom_field_update(yak, field_updated),
        }
    }

    fn apply_custom_field_update(yak: &mut Yak, field_updated: FieldUpdatedEvent) {
        if field_updated.content.is_empty() {
            yak.fields.remove(&field_updated.field_name);
        } else {
            yak.fields
                .insert(field_updated.field_name, field_updated.content);
        }
    }

    fn apply_blocker_added_or_updated(&mut self, target: YakId, blocker: Blocker) {
        self.blockers
            .entry(target)
            .or_default()
            .insert(blocker.source, blocker.reason);
    }

    fn apply_blocker_removed(&mut self, e: BlockerRemovedEvent) {
        if let Some(blockers) = self.blockers.get_mut(&e.target) {
            blockers.remove(&e.source);
            if blockers.is_empty() {
                self.blockers.remove(&e.target);
            }
        }
    }

    fn apply_compacted(&mut self, snapshot: YakMapSnapshot) {
        self.yaks.clear();
        self.blockers.clear();
        for mut yak in snapshot.yaks {
            if yak.state == YakState::Blocked {
                yak.state = YakState::Todo;
                self.blockers.entry(yak.id.clone()).or_default().insert(
                    BlockerSource::Manual,
                    Some(MIGRATED_BLOCKED_REASON.to_string()),
                );
            }
            if let Some(reason) = yak.fields.remove(MANUAL_BLOCKER_FIELD) {
                if !reason.trim().is_empty() {
                    self.blockers
                        .entry(yak.id.clone())
                        .or_default()
                        .insert(BlockerSource::Manual, Some(reason));
                }
            }
            self.yaks.insert(yak.id.clone(), yak);
        }
        for blocker in snapshot.blockers {
            self.blockers
                .entry(blocker.target)
                .or_default()
                .insert(BlockerSource::Yak(blocker.blocker), blocker.reason);
        }
        for blocker in snapshot.manual_blockers {
            self.blockers
                .entry(blocker.target)
                .or_default()
                .insert(BlockerSource::Manual, Some(blocker.reason));
        }
    }

    pub fn snapshot(&self, removed_yak_ids: Vec<YakId>) -> YakMapSnapshot {
        let mut yaks: Vec<_> = self.yaks.values().cloned().collect();
        yaks.sort_by_key(|yak| yak.id.as_str().to_string());

        let mut blockers: Vec<_> = self
            .blockers
            .iter()
            .flat_map(|(target, blockers)| {
                blockers.iter().filter_map(|(source, reason)| match source {
                    BlockerSource::Yak(blocker) => Some(YakBlockerSnapshot {
                        target: target.clone(),
                        blocker: blocker.clone(),
                        reason: reason.clone(),
                    }),
                    BlockerSource::Manual => None,
                })
            })
            .collect();
        blockers.sort_by(|a, b| {
            a.target
                .as_str()
                .cmp(b.target.as_str())
                .then_with(|| a.blocker.as_str().cmp(b.blocker.as_str()))
        });

        let mut manual_blockers: Vec<_> = self
            .blockers
            .iter()
            .filter_map(|(target, blockers)| {
                blockers.get(&BlockerSource::Manual).and_then(|reason| {
                    reason.clone().map(|reason| ManualBlockerSnapshot {
                        target: target.clone(),
                        reason,
                    })
                })
            })
            .collect();
        manual_blockers.sort_by(|a, b| a.target.as_str().cmp(b.target.as_str()));

        YakMapSnapshot {
            yaks,
            removed_yak_ids,
            blockers,
            manual_blockers,
        }
    }

    pub fn take_events(&mut self) -> Vec<YakEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Build the full display name for a yak by walking up the parent chain.
    fn build_display_name(&self, id: &YakId) -> String {
        let mut parts = Vec::new();
        let mut current_id = Some(id.clone());

        while let Some(ref cid) = current_id {
            if let Some(entry) = self.yaks.get(cid) {
                parts.push(entry.name.to_string());
                current_id = entry.parent_id.clone();
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
            .filter(|(_, entry)| entry.parent_id.as_ref() == Some(parent_id))
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

    fn is_descendant_of(&self, descendant: &YakId, ancestor: &YakId) -> bool {
        self.get_ancestor_ids(descendant)
            .iter()
            .any(|id| id == ancestor)
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

        for (id, entry) in &self.yaks {
            // Skip the yak itself (for rename case)
            if let Some(sid) = self_id {
                if id == sid {
                    continue;
                }
            }

            // Only check siblings (same parent)
            if &entry.parent_id != parent_id {
                continue;
            }

            let sibling_slug = slugify(entry.name.as_str());
            if sibling_slug.as_str() == new_slug.as_str() {
                let msg = match parent_id {
                    Some(pid) => {
                        let parent_display = self.build_display_name(pid);
                        format!(
                            "A yak named \"{}\" already exists \
                             under \"{}\" with the same slug \
                             \"{}\". Try a more distinct name.",
                            entry.name, parent_display, new_slug
                        )
                    }
                    None => {
                        format!(
                            "A yak named \"{}\" already exists \
                             with the same slug \"{}\". \
                             Try a more distinct name.",
                            entry.name, new_slug
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
        let name = name.into();

        // Validate and parse state if provided
        let initial_state = if let Some(ref s) = state {
            s.parse::<YakState>().map_err(|e| anyhow::anyhow!(e))?
        } else {
            YakState::Todo
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
            Yak {
                id: id.clone(),
                name: name.clone(),
                parent_id: parent_id.clone(),
                state: initial_state,
                context: context.clone(),
                fields: HashMap::new(),
                tags: vec![],
                created_by: self.metadata.author.clone(),
                created_at: self.metadata.timestamp,
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

        if initial_state != YakState::Todo {
            self.pending_events.push(YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: id.clone(),
                    field_name: ".state".to_string(),
                    content: initial_state.to_string(),
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
        let new_state: YakState = state.parse().map_err(|e: String| anyhow::anyhow!(e))?;

        self.ensure_exists(&id)?;

        // Validate blockers and children if marking done
        if new_state == YakState::Done {
            self.validate_no_active_blockers(&id)?;
            self.validate_children_complete(&id)?;
        }

        // Capture old state before updating
        let old_state = self.yaks.get(&id).unwrap().state;
        let transitioning_from_done = old_state == YakState::Done && new_state != YakState::Done;

        if new_state == YakState::Done {
            self.remove_blocked_by(&id);
        }

        // Update this yak
        let yak = self.yaks.get_mut(&id).unwrap();
        yak.state = new_state;
        self.pending_events.push(YakEvent::FieldUpdated(
            FieldUpdatedEvent {
                id: id.clone(),
                field_name: ".state".to_string(),
                content: new_state.to_string(),
            },
            self.metadata.clone(),
        ));

        // Demote done ancestors if transitioning from done
        if transitioning_from_done {
            self.demote_done_ancestors_to_todo(&id);
        }

        Ok(())
    }

    pub fn is_ready(&self, id: &YakId) -> Result<bool> {
        self.ensure_exists(id)?;

        let yak = self.yaks.get(id).unwrap();
        if yak.state != YakState::Todo {
            return Ok(false);
        }

        if !self.active_blockers(id).is_empty() {
            return Ok(false);
        }

        Ok(self
            .find_children_of(id)
            .iter()
            .all(|cid| self.yaks.get(cid).unwrap().state == YakState::Done))
    }

    pub fn ensure_ready_to_start(&self, id: &YakId) -> Result<()> {
        self.ensure_exists(id)?;

        let display = self.build_display_name(id);
        let yak = self.yaks.get(id).unwrap();
        if yak.state != YakState::Todo {
            anyhow::bail!(
                "cannot start '{}' - it is not ready (state is {})",
                display,
                yak.state
            );
        }

        let active_blockers = self.active_blockers(id);
        if !active_blockers.is_empty() {
            let blocker_names = self.format_active_blockers(&active_blockers);
            anyhow::bail!(
                "cannot start '{}' - it is not ready (blocked by {})",
                display,
                blocker_names
            );
        }

        let incomplete_children = self
            .find_children_of(id)
            .into_iter()
            .filter(|cid| self.yaks.get(cid).unwrap().state != YakState::Done)
            .map(|cid| self.build_display_name(&cid))
            .collect::<Vec<_>>();
        if !incomplete_children.is_empty() {
            anyhow::bail!(
                "cannot start '{}' - it is not ready (incomplete children: {})",
                display,
                incomplete_children.join(", ")
            );
        }

        Ok(())
    }

    fn remove_blocked_by(&mut self, blocker_id: &YakId) {
        let source = BlockerSource::Yak(blocker_id.clone());
        let relationships: Vec<(YakId, YakId)> = self
            .blockers
            .iter()
            .filter(|(_, blockers)| blockers.contains_key(&source))
            .map(|(target, _)| (target.clone(), blocker_id.clone()))
            .collect();

        self.remove_explicit_blocker_relationships(relationships);
    }

    fn remove_blockers_touching(&mut self, id: &YakId) {
        let mut relationships: Vec<(YakId, YakId)> = self
            .blockers
            .iter()
            .flat_map(|(target, blockers)| {
                blockers.keys().filter_map(move |source| match source {
                    BlockerSource::Yak(blocker) if target == id || blocker == id => {
                        Some((target.clone(), blocker.clone()))
                    }
                    _ => None,
                })
            })
            .collect();
        relationships.sort_by(|(a_target, a_blocker), (b_target, b_blocker)| {
            a_target
                .as_str()
                .cmp(b_target.as_str())
                .then_with(|| a_blocker.as_str().cmp(b_blocker.as_str()))
        });

        self.remove_explicit_blocker_relationships(relationships);
    }

    fn remove_explicit_blocker_relationships(&mut self, relationships: Vec<(YakId, YakId)>) {
        for (target, blocker) in relationships {
            if let Some(blockers) = self.blockers.get_mut(&target) {
                let source = BlockerSource::Yak(blocker);
                if blockers.remove(&source).is_some() {
                    if blockers.is_empty() {
                        self.blockers.remove(&target);
                    }
                    self.pending_events.push(YakEvent::BlockerRemoved(
                        BlockerRemovedEvent { target, source },
                        self.metadata.clone(),
                    ));
                }
            }
        }
    }

    fn validate_no_active_blockers(&self, id: &YakId) -> Result<()> {
        let active_blockers = self.active_blockers(id);
        if !active_blockers.is_empty() {
            let display = self.build_display_name(id);
            let blocker_names = self.format_active_blockers(&active_blockers);
            anyhow::bail!(
                "cannot mark '{}' as done - it is blocked by {}",
                display,
                blocker_names
            );
        }

        Ok(())
    }

    fn format_active_blockers(&self, active_blockers: &[BlockingDependency]) -> String {
        active_blockers
            .iter()
            .map(|blocker| match &blocker.source {
                BlockerSource::Yak(id) => self.build_display_name(id),
                BlockerSource::Manual => blocker
                    .reason
                    .clone()
                    .unwrap_or_else(|| "manual blocker".to_string()),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn validate_children_complete(&self, parent_id: &YakId) -> Result<()> {
        let children = self.find_children_of(parent_id);

        let incomplete = children
            .iter()
            .any(|cid| self.yaks.get(cid).unwrap().state != YakState::Done);

        if incomplete {
            let display = self.build_display_name(parent_id);
            anyhow::bail!(
                "cannot mark '{}' as done - it has incomplete children",
                display
            );
        }

        Ok(())
    }

    fn demote_done_ancestors_to_todo(&mut self, id: &YakId) {
        for ancestor_id in self.get_ancestor_ids(id) {
            if let Some(parent) = self.yaks.get_mut(&ancestor_id) {
                if parent.state == YakState::Done {
                    parent.state = YakState::Todo;
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

    pub fn active_blockers(&self, id: &YakId) -> Vec<BlockingDependency> {
        let mut blockers: Vec<BlockingDependency> = self
            .blockers
            .get(id)
            .into_iter()
            .flat_map(|blockers| blockers.iter())
            .map(|(source, reason)| BlockingDependency {
                source: source.clone(),
                reason: reason.clone(),
            })
            .collect();
        blockers.sort_by(|a, b| a.source.sort_key().cmp(&b.source.sort_key()));
        blockers
    }

    fn path_to_blocker(&self, start: &YakId, goal: &YakId) -> Option<BlockerPath> {
        let mut stack = vec![(start.clone(), vec![start.clone()])];
        let mut visited = HashSet::new();

        while let Some((current, path)) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if &current == goal {
                return Some(BlockerPath { yaks: path });
            }

            let mut explicit_targets: Vec<_> = self
                .blockers
                .iter()
                .filter(|(_, blockers)| blockers.contains_key(&BlockerSource::Yak(current.clone())))
                .map(|(target, _)| target.clone())
                .collect();
            explicit_targets.sort_by(|a, b| b.as_str().cmp(a.as_str()));
            for target in explicit_targets {
                let mut next_path = path.clone();
                next_path.push(target.clone());
                stack.push((target, next_path));
            }

            if let Some(parent) = self
                .yaks
                .get(&current)
                .and_then(|yak| yak.parent_id.clone())
            {
                let mut next_path = path;
                next_path.push(parent.clone());
                stack.push((parent, next_path));
            }
        }

        None
    }

    fn format_circular_dependency_path(&self, blocker: &YakId, path: &[YakId]) -> String {
        let mut names = Vec::with_capacity(path.len() + 2);
        names.push(self.build_display_name(blocker));
        names.extend(path.iter().map(|id| self.build_display_name(id)));
        names.push(self.build_display_name(blocker));
        names.join(" -> ")
    }

    fn parent_after_move(
        &self,
        id: &YakId,
        moved_id: &YakId,
        new_parent_id: &Option<YakId>,
    ) -> Option<YakId> {
        if id == moved_id {
            new_parent_id.clone()
        } else {
            self.yaks.get(id).and_then(|yak| yak.parent_id.clone())
        }
    }

    fn ancestor_ids_after_move(
        &self,
        id: &YakId,
        moved_id: &YakId,
        new_parent_id: &Option<YakId>,
    ) -> Vec<YakId> {
        let mut ancestors = Vec::new();
        let mut visited = HashSet::new();
        let mut current_id = self.parent_after_move(id, moved_id, new_parent_id);

        while let Some(pid) = current_id {
            if !visited.insert(pid.clone()) {
                break;
            }
            ancestors.push(pid.clone());
            current_id = self.parent_after_move(&pid, moved_id, new_parent_id);
        }

        ancestors
    }

    fn subtree_ids(&self, root_id: &YakId) -> Vec<YakId> {
        let mut ids = vec![root_id.clone()];
        let mut stack = self.find_children_of(root_id);
        stack.sort_by(|a, b| b.as_str().cmp(a.as_str()));

        while let Some(id) = stack.pop() {
            let mut children = self.find_children_of(&id);
            children.sort_by(|a, b| b.as_str().cmp(a.as_str()));
            stack.extend(children);
            ids.push(id);
        }

        ids
    }

    fn explicit_blockers_replaced_by_hierarchy_after_move(
        &self,
        id: &YakId,
        new_parent_id: &Option<YakId>,
    ) -> Vec<(YakId, YakId)> {
        let mut relationships = Vec::new();

        for blocker in self.subtree_ids(id) {
            for target in self.ancestor_ids_after_move(&blocker, id, new_parent_id) {
                if self.blockers.get(&target).is_some_and(|blockers| {
                    blockers.contains_key(&BlockerSource::Yak(blocker.clone()))
                }) {
                    relationships.push((target, blocker.clone()));
                }
            }
        }

        relationships.sort_by(|(a_target, a_blocker), (b_target, b_blocker)| {
            a_target
                .as_str()
                .cmp(b_target.as_str())
                .then_with(|| a_blocker.as_str().cmp(b_blocker.as_str()))
        });
        relationships
    }

    fn path_to_blocker_after_move(
        &self,
        start: &YakId,
        goal: &YakId,
        moved_id: &YakId,
        new_parent_id: &Option<YakId>,
        excluded_explicit_relationships: &HashSet<(YakId, YakId)>,
    ) -> Option<BlockerPath> {
        let mut stack = vec![(start.clone(), vec![start.clone()])];
        let mut visited = HashSet::new();

        while let Some((current, path)) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if &current == goal {
                return Some(BlockerPath { yaks: path });
            }

            let mut explicit_targets: Vec<_> = self
                .blockers
                .iter()
                .filter(|(target, blockers)| {
                    blockers.contains_key(&BlockerSource::Yak(current.clone()))
                        && !excluded_explicit_relationships
                            .contains(&((*target).clone(), current.clone()))
                })
                .map(|(target, _)| target.clone())
                .collect();
            explicit_targets.sort_by(|a, b| b.as_str().cmp(a.as_str()));
            for target in explicit_targets {
                let mut next_path = path.clone();
                next_path.push(target.clone());
                stack.push((target, next_path));
            }

            if let Some(parent) = self.parent_after_move(&current, moved_id, new_parent_id) {
                let mut next_path = path;
                next_path.push(parent.clone());
                stack.push((parent, next_path));
            }
        }

        None
    }

    fn validate_move_preserves_acyclic_blocker_graph(
        &self,
        id: &YakId,
        new_parent_id: &Option<YakId>,
        explicit_relationships_to_remove: &[(YakId, YakId)],
    ) -> Result<()> {
        let Some(new_parent) = new_parent_id else {
            return Ok(());
        };

        let excluded: HashSet<_> = explicit_relationships_to_remove.iter().cloned().collect();
        for subtree_id in self.subtree_ids(id) {
            if let Some(path) = self.path_to_blocker_after_move(
                new_parent,
                &subtree_id,
                id,
                new_parent_id,
                &excluded,
            ) {
                anyhow::bail!(
                    "moving '{}' under '{}' would create circular dependency: {}",
                    self.build_display_name(id),
                    self.build_display_name(new_parent),
                    self.format_circular_dependency_path(&subtree_id, &path.yaks)
                );
            }
        }

        Ok(())
    }

    pub fn add_blocker(
        &mut self,
        target: YakId,
        blocker: YakId,
        reason: Option<String>,
    ) -> Result<AddBlockerOutcome> {
        self.ensure_exists(&target)?;
        self.ensure_exists(&blocker)?;
        if target == blocker {
            anyhow::bail!(
                "yak '{}' cannot block itself",
                self.build_display_name(&target)
            );
        }
        let requested_reason =
            reason.map(|reason| crate::domain::events::blocker::normalize_reason(Some(reason)));

        let source = BlockerSource::Yak(blocker.clone());
        let existing = self
            .blockers
            .get(&target)
            .and_then(|blockers| blockers.get(&source));
        let (event, outcome, new_reason) = if let Some(existing_reason) = existing {
            let Some(new_reason) = requested_reason else {
                return Ok(AddBlockerOutcome::AlreadyExplicit);
            };
            if existing_reason == &new_reason {
                return Ok(AddBlockerOutcome::AlreadyExplicit);
            }
            (
                YakEvent::BlockerUpdated(
                    BlockerUpdatedEvent {
                        target: target.clone(),
                        blocker: Blocker {
                            source: source.clone(),
                            reason: new_reason.clone(),
                        },
                    },
                    self.metadata.clone(),
                ),
                AddBlockerOutcome::Updated,
                new_reason,
            )
        } else if self.is_descendant_of(&blocker, &target) {
            return Ok(AddBlockerOutcome::AlreadyImpliedByHierarchy);
        } else {
            if let Some(path) = self.path_to_blocker(&target, &blocker) {
                anyhow::bail!(
                    "adding '{}' as a blocker for '{}' would create circular dependency: {}",
                    self.build_display_name(&blocker),
                    self.build_display_name(&target),
                    self.format_circular_dependency_path(&blocker, &path.yaks)
                );
            }
            let new_reason = requested_reason.unwrap_or(None);
            (
                YakEvent::BlockerAdded(
                    BlockerAddedEvent {
                        target: target.clone(),
                        blocker: Blocker {
                            source: source.clone(),
                            reason: new_reason.clone(),
                        },
                    },
                    self.metadata.clone(),
                ),
                AddBlockerOutcome::Added,
                new_reason,
            )
        };
        self.blockers
            .entry(target)
            .or_default()
            .insert(source, new_reason);
        self.pending_events.push(event);
        Ok(outcome)
    }

    pub fn add_manual_blocker(
        &mut self,
        target: YakId,
        reason: String,
    ) -> Result<AddBlockerOutcome> {
        self.ensure_exists(&target)?;
        anyhow::ensure!(
            !reason.trim().is_empty(),
            "manual blockers require a non-empty --reason"
        );
        let reason = reason.trim().to_string();
        let source = BlockerSource::Manual;
        let existing = self
            .blockers
            .get(&target)
            .and_then(|blockers| blockers.get(&source))
            .cloned()
            .flatten();
        match existing {
            Some(existing) if existing == reason => Ok(AddBlockerOutcome::AlreadyExplicit),
            Some(_) => {
                self.blockers
                    .entry(target.clone())
                    .or_default()
                    .insert(source.clone(), Some(reason.clone()));
                self.pending_events.push(YakEvent::BlockerUpdated(
                    BlockerUpdatedEvent {
                        target,
                        blocker: Blocker {
                            source,
                            reason: Some(reason),
                        },
                    },
                    self.metadata.clone(),
                ));
                Ok(AddBlockerOutcome::Updated)
            }
            None => {
                self.blockers
                    .entry(target.clone())
                    .or_default()
                    .insert(source.clone(), Some(reason.clone()));
                self.pending_events.push(YakEvent::BlockerAdded(
                    BlockerAddedEvent {
                        target,
                        blocker: Blocker {
                            source,
                            reason: Some(reason),
                        },
                    },
                    self.metadata.clone(),
                ));
                Ok(AddBlockerOutcome::Added)
            }
        }
    }

    pub fn remove_manual_blocker(&mut self, target: YakId) -> Result<RemoveBlockerOutcome> {
        self.ensure_exists(&target)?;
        let source = BlockerSource::Manual;
        if let Some(blockers) = self.blockers.get_mut(&target) {
            if blockers.remove(&source).is_some() {
                if blockers.is_empty() {
                    self.blockers.remove(&target);
                }
                self.pending_events.push(YakEvent::BlockerRemoved(
                    BlockerRemovedEvent { target, source },
                    self.metadata.clone(),
                ));
                return Ok(RemoveBlockerOutcome::Removed);
            }
        }
        Ok(RemoveBlockerOutcome::NotPresent)
    }

    pub fn remove_blocker(
        &mut self,
        target: YakId,
        blocker: YakId,
    ) -> Result<RemoveBlockerOutcome> {
        self.ensure_exists(&target)?;
        self.ensure_exists(&blocker)?;
        let source = BlockerSource::Yak(blocker);
        if let Some(blockers) = self.blockers.get_mut(&target) {
            if blockers.remove(&source).is_some() {
                if blockers.is_empty() {
                    self.blockers.remove(&target);
                }
                self.pending_events.push(YakEvent::BlockerRemoved(
                    BlockerRemovedEvent { target, source },
                    self.metadata.clone(),
                ));
                return Ok(RemoveBlockerOutcome::Removed);
            }
        }
        Ok(RemoveBlockerOutcome::NotPresent)
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

        self.remove_blockers_touching(&id);
        self.yaks.remove(&id);
        self.pending_events.push(YakEvent::Removed(
            RemovedEvent { id },
            self.metadata.clone(),
        ));

        Ok(())
    }

    pub fn prune(&mut self, exclude_tag: Option<&str>) -> Result<()> {
        loop {
            let done_leaves: Vec<YakId> = self
                .yaks
                .iter()
                .filter(|(id, entry)| {
                    entry.state == YakState::Done
                        && self.find_children_of(id).is_empty()
                        && !exclude_tag.is_some_and(|tag| entry.tags.contains(&tag.to_string()))
                })
                .map(|(id, _)| id.clone())
                .collect();

            if done_leaves.is_empty() {
                break;
            }

            for id in done_leaves {
                self.remove_blockers_touching(&id);
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

        let explicit_relationships_to_remove =
            self.explicit_blockers_replaced_by_hierarchy_after_move(&id, &new_parent_id);
        self.validate_move_preserves_acyclic_blocker_graph(
            &id,
            &new_parent_id,
            &explicit_relationships_to_remove,
        )?;

        self.remove_explicit_blocker_relationships(explicit_relationships_to_remove);

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
    use std::collections::HashMap;

    #[test]
    fn legacy_blocked_state_replays_as_todo_with_manual_blocker() {
        let id = YakId::from("legacy-a1b2");
        let map = YakMap::from_events(
            vec![
                YakEvent::Added(
                    AddedEvent {
                        name: Name::from("legacy"),
                        id: id.clone(),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ),
                YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: id.clone(),
                        field_name: ".state".to_string(),
                        content: "blocked".to_string(),
                    },
                    EventMetadata::default_legacy(),
                ),
            ],
            EventMetadata::default_legacy(),
        )
        .unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().state, YakState::Todo);
        assert_eq!(map.active_blockers(&id)[0].source, BlockerSource::Manual);
        assert_eq!(
            map.active_blockers(&id)[0].reason.as_deref(),
            Some(MIGRATED_BLOCKED_REASON)
        );
        assert!(!map.is_ready(&id).unwrap());
    }

    #[test]
    fn non_blocked_state_change_clears_migrated_manual_blocker() {
        let id = YakId::from("legacy-a1b2");
        let map = YakMap::from_events(
            vec![
                YakEvent::Added(
                    AddedEvent {
                        name: Name::from("legacy"),
                        id: id.clone(),
                        parent_id: None,
                    },
                    EventMetadata::default_legacy(),
                ),
                YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: id.clone(),
                        field_name: ".state".to_string(),
                        content: "blocked".to_string(),
                    },
                    EventMetadata::default_legacy(),
                ),
                YakEvent::FieldUpdated(
                    FieldUpdatedEvent {
                        id: id.clone(),
                        field_name: ".state".to_string(),
                        content: "todo".to_string(),
                    },
                    EventMetadata::default_legacy(),
                ),
            ],
            EventMetadata::default_legacy(),
        )
        .unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().state, YakState::Todo);
        assert!(map.active_blockers(&id).is_empty());
        assert!(map.is_ready(&id).unwrap());
    }

    #[test]
    fn compacted_legacy_blocked_state_replays_as_todo_with_manual_blocker() {
        let id = YakId::from("legacy-a1b2");
        let snapshot = Yak {
            id: id.clone(),
            name: Name::from("legacy"),
            parent_id: None,
            state: YakState::Blocked,
            context: None,
            fields: HashMap::new(),
            tags: vec![],
            created_by: crate::domain::event_metadata::Author::unknown(),
            created_at: crate::domain::event_metadata::Timestamp::zero(),
        };

        let map = YakMap::from_events(
            vec![YakEvent::Compacted(
                YakMapSnapshot::legacy(vec![snapshot], vec![]),
                EventMetadata::default_legacy(),
            )],
            EventMetadata::default_legacy(),
        )
        .unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().state, YakState::Todo);
        assert_eq!(
            map.active_blockers(&id)[0].reason.as_deref(),
            Some(MIGRATED_BLOCKED_REASON)
        );
    }

    #[test]
    fn compacted_manual_blocker_field_replays_manual_blocker() {
        let id = YakId::from("manual-a1b2");
        let mut fields = HashMap::new();
        fields.insert(MANUAL_BLOCKER_FIELD.to_string(), "waiting".to_string());
        let snapshot = Yak {
            id: id.clone(),
            name: Name::from("manual"),
            parent_id: None,
            state: YakState::Todo,
            context: None,
            fields,
            tags: vec![],
            created_by: crate::domain::event_metadata::Author::unknown(),
            created_at: crate::domain::event_metadata::Timestamp::zero(),
        };

        let map = YakMap::from_events(
            vec![YakEvent::Compacted(
                YakMapSnapshot::legacy(vec![snapshot], vec![]),
                EventMetadata::default_legacy(),
            )],
            EventMetadata::default_legacy(),
        )
        .unwrap();

        assert_eq!(
            map.active_blockers(&id)[0].reason.as_deref(),
            Some("waiting")
        );
        assert!(!map
            .yaks
            .get(&id)
            .unwrap()
            .fields
            .contains_key(MANUAL_BLOCKER_FIELD));
    }

    #[test]
    fn compacted_snapshot_replays_explicit_and_manual_blockers() {
        let target = YakId::from("deploy-a1b2");
        let blocker = YakId::from("security-review-c3d4");
        let yaks = vec![
            Yak {
                id: target.clone(),
                name: Name::from("deploy"),
                parent_id: None,
                state: YakState::Todo,
                context: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: crate::domain::event_metadata::Author::unknown(),
                created_at: crate::domain::event_metadata::Timestamp::zero(),
            },
            Yak {
                id: blocker.clone(),
                name: Name::from("security review"),
                parent_id: None,
                state: YakState::Todo,
                context: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: crate::domain::event_metadata::Author::unknown(),
                created_at: crate::domain::event_metadata::Timestamp::zero(),
            },
        ];
        let snapshot = YakMapSnapshot {
            yaks,
            removed_yak_ids: vec![],
            blockers: vec![YakBlockerSnapshot {
                target: target.clone(),
                blocker: blocker.clone(),
                reason: Some("waiting for approval".to_string()),
            }],
            manual_blockers: vec![ManualBlockerSnapshot {
                target: target.clone(),
                reason: "manual hold".to_string(),
            }],
        };

        let map = YakMap::from_events(
            vec![YakEvent::Compacted(
                snapshot,
                EventMetadata::default_legacy(),
            )],
            EventMetadata::default_legacy(),
        )
        .unwrap();

        let active_blockers = map.active_blockers(&target);
        assert!(active_blockers.iter().any(|b| {
            b.source == BlockerSource::Yak(blocker.clone())
                && b.reason.as_deref() == Some("waiting for approval")
        }));
        assert!(active_blockers.iter().any(|b| {
            b.source == BlockerSource::Manual && b.reason.as_deref() == Some("manual hold")
        }));
        assert!(!map.is_ready(&target).unwrap());
    }

    #[test]
    fn add_blocker_normalizes_empty_reason_to_none() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked yak", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocking yak", None, None, None, None, vec![])
            .unwrap();

        let outcome = map
            .add_blocker(target.clone(), blocker.clone(), Some(String::new()))
            .unwrap();

        assert_eq!(outcome, AddBlockerOutcome::Added);
        assert_eq!(map.active_blockers(&target)[0].reason, None);
        let events = map.take_events();
        let YakEvent::BlockerAdded(event, _) = events.last().unwrap() else {
            panic!("expected BlockerAdded event");
        };
        assert_eq!(event.blocker.reason, None);
    }

    #[test]
    fn add_blocker_with_same_reason_is_noop() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked yak", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocking yak", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), Some("same".to_string()))
            .unwrap();
        map.take_events();

        let outcome = map
            .add_blocker(target.clone(), blocker, Some("same".to_string()))
            .unwrap();

        assert_eq!(outcome, AddBlockerOutcome::AlreadyExplicit);
        assert_eq!(
            map.active_blockers(&target)[0].reason,
            Some("same".to_string())
        );
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_without_reason_preserves_existing_reason_as_noop() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked yak", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocking yak", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(
            target.clone(),
            blocker.clone(),
            Some("existing".to_string()),
        )
        .unwrap();
        map.take_events();

        let outcome = map.add_blocker(target.clone(), blocker, None).unwrap();

        assert_eq!(outcome, AddBlockerOutcome::AlreadyExplicit);
        assert_eq!(
            map.active_blockers(&target)[0].reason,
            Some("existing".to_string())
        );
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_with_empty_reason_clears_existing_reason() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked yak", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocking yak", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(
            target.clone(),
            blocker.clone(),
            Some("existing".to_string()),
        )
        .unwrap();
        map.take_events();

        let outcome = map
            .add_blocker(target.clone(), blocker, Some(String::new()))
            .unwrap();

        assert_eq!(outcome, AddBlockerOutcome::Updated);
        assert_eq!(map.active_blockers(&target)[0].reason, None);
        let events = map.take_events();
        assert!(matches!(
            events.as_slice(),
            [YakEvent::BlockerUpdated(_, _)]
        ));
    }

    #[test]
    fn add_blocker_with_empty_reason_when_reason_absent_is_noop() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked yak", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocking yak", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        let outcome = map
            .add_blocker(target.clone(), blocker, Some(String::new()))
            .unwrap();

        assert_eq!(outcome, AddBlockerOutcome::AlreadyExplicit);
        assert_eq!(map.active_blockers(&target)[0].reason, None);
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_with_changed_reason_updates() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked yak", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocking yak", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), Some("old".to_string()))
            .unwrap();
        map.take_events();

        let outcome = map
            .add_blocker(target.clone(), blocker, Some("new".to_string()))
            .unwrap();

        assert_eq!(outcome, AddBlockerOutcome::Updated);
        assert_eq!(
            map.active_blockers(&target)[0].reason,
            Some("new".to_string())
        );
        let events = map.take_events();
        assert!(matches!(
            events.as_slice(),
            [YakEvent::BlockerUpdated(_, _)]
        ));
    }

    #[test]
    fn add_blocker_implied_by_hierarchy_is_noop() {
        let mut map = YakMap::new();
        let parent = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child = map
            .add_yak("child", Some(parent.clone()), None, None, None, vec![])
            .unwrap();
        map.take_events();

        let outcome = map.add_blocker(parent.clone(), child, None).unwrap();

        assert_eq!(outcome, AddBlockerOutcome::AlreadyImpliedByHierarchy);
        assert!(map.active_blockers(&parent).is_empty());
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_rejects_self_blocking() {
        let mut map = YakMap::new();
        let yak = map.add_yak("yak", None, None, None, None, vec![]).unwrap();
        map.take_events();

        let err = map
            .add_blocker(yak.clone(), yak, None)
            .unwrap_err()
            .to_string();

        assert!(err.contains("cannot block itself"));
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_rejects_mutual_explicit_cycle() {
        let mut map = YakMap::new();
        let a = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let b = map.add_yak("b", None, None, None, None, vec![]).unwrap();
        map.add_blocker(a.clone(), b.clone(), None).unwrap();
        map.take_events();

        let err = map.add_blocker(b, a, None).unwrap_err().to_string();

        assert!(err.contains("would create circular dependency"));
        assert!(err.contains("a -> b -> a"));
        assert!(!err.contains("through hierarchy"));
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_rejects_longer_explicit_cycle() {
        let mut map = YakMap::new();
        let a = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let b = map.add_yak("b", None, None, None, None, vec![]).unwrap();
        let c = map.add_yak("c", None, None, None, None, vec![]).unwrap();
        map.add_blocker(a.clone(), b.clone(), None).unwrap();
        map.add_blocker(b.clone(), c.clone(), None).unwrap();
        map.take_events();

        let err = map.add_blocker(c, a, None).unwrap_err().to_string();

        assert!(err.contains("would create circular dependency"));
        assert!(err.contains("a -> c -> b -> a"));
        assert!(!err.contains("through hierarchy"));
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_rejects_ancestor_blocking_descendant_through_hierarchy() {
        let mut map = YakMap::new();
        let parent = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child = map
            .add_yak("child", Some(parent.clone()), None, None, None, vec![])
            .unwrap();
        map.take_events();

        let err = map
            .add_blocker(child, parent, None)
            .unwrap_err()
            .to_string();

        assert!(err.contains("would create circular dependency"));
        assert!(!err.contains("through hierarchy"));
        assert!(err.contains("parent -> parent/child -> parent"));
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_rejects_multi_ancestor_hierarchy_cycle() {
        let mut map = YakMap::new();
        let parent = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child = map
            .add_yak("child", Some(parent.clone()), None, None, None, vec![])
            .unwrap();
        let grandchild = map
            .add_yak("grandchild", Some(child), None, None, None, vec![])
            .unwrap();
        map.take_events();

        let err = map
            .add_blocker(grandchild, parent, None)
            .unwrap_err()
            .to_string();

        assert!(err.contains("would create circular dependency"));
        assert!(!err.contains("through hierarchy"));
        assert!(err.contains("parent -> parent/child/grandchild -> parent/child -> parent"));
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn add_blocker_rejects_explicit_cycle_through_hierarchy() {
        let mut map = YakMap::new();
        let parent = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child = map
            .add_yak("child", Some(parent.clone()), None, None, None, vec![])
            .unwrap();
        let other = map
            .add_yak("other", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(child, other.clone(), None).unwrap();
        map.take_events();

        let err = map
            .add_blocker(other, parent, None)
            .unwrap_err()
            .to_string();

        assert!(err.contains("would create circular dependency"));
        assert!(!err.contains("through hierarchy"));
        assert!(err.contains("parent -> other -> parent/child -> parent"));
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn remove_absent_blocker_is_noop() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked yak", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocking yak", None, None, None, None, vec![])
            .unwrap();
        map.take_events();

        let outcome = map.remove_blocker(target, blocker).unwrap();

        assert_eq!(outcome, RemoveBlockerOutcome::NotPresent);
        assert!(map.take_events().is_empty());
    }

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
        use crate::domain::Yak;

        struct MockStore;

        impl ReadYakStore for MockStore {
            fn get_yak(&self, _id: &YakId) -> Result<Yak> {
                anyhow::bail!("empty")
            }
            fn list_yaks(&self) -> Result<Vec<Yak>> {
                Ok(vec![])
            }
            fn fuzzy_find_yak_id(&self, _query: &str) -> Result<YakId> {
                anyhow::bail!("empty")
            }
            fn read_field(&self, _id: &YakId, _field_name: &str) -> Result<String> {
                anyhow::bail!("Not implemented")
            }

            fn list_blockers(&self) -> Result<Vec<crate::domain::YakBlockerSnapshot>> {
                Ok(Vec::new())
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
        use crate::domain::Yak;

        struct MockStore {
            yaks: Vec<Yak>,
        }

        impl ReadYakStore for MockStore {
            fn get_yak(&self, id: &YakId) -> Result<Yak> {
                self.yaks
                    .iter()
                    .find(|y| y.id == *id)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Yak not found"))
            }

            fn list_yaks(&self) -> Result<Vec<Yak>> {
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

            fn list_blockers(&self) -> Result<Vec<crate::domain::YakBlockerSnapshot>> {
                Ok(Vec::new())
            }
        }

        use crate::domain::event_metadata::{Author, Timestamp};
        let store = MockStore {
            yaks: vec![
                Yak {
                    id: YakId::from("test1-aaaa"),
                    name: Name::from("test1"),
                    parent_id: None,
                    state: YakState::Todo,
                    context: Some("context1".to_string()),
                    fields: std::collections::HashMap::new(),
                    tags: vec![],
                    created_by: Author::unknown(),
                    created_at: Timestamp::zero(),
                },
                Yak {
                    id: YakId::from("test2-bbbb"),
                    name: Name::from("test2"),
                    parent_id: None,
                    state: YakState::Wip,
                    context: None,
                    fields: std::collections::HashMap::new(),
                    tags: vec![],
                    created_by: Author::unknown(),
                    created_at: Timestamp::zero(),
                },
            ],
        };
        let map = YakMap::from_store(&store, EventMetadata::default_legacy()).unwrap();

        assert_eq!(map.yaks.len(), 2);
        assert_eq!(
            map.yaks.get(&YakId::from("test1-aaaa")).unwrap().state,
            YakState::Todo
        );
        assert_eq!(
            map.yaks.get(&YakId::from("test1-aaaa")).unwrap().context,
            Some("context1".to_string())
        );
        assert_eq!(
            map.yaks.get(&YakId::from("test2-bbbb")).unwrap().state,
            YakState::Wip
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
        use crate::domain::Yak;

        struct MockStore;

        impl ReadYakStore for MockStore {
            fn get_yak(&self, _id: &YakId) -> Result<Yak> {
                anyhow::bail!("Not needed")
            }

            fn list_yaks(&self) -> Result<Vec<Yak>> {
                use crate::domain::event_metadata::{Author, Timestamp};
                Ok(vec![
                    Yak {
                        id: YakId::from("parent-aaaa"),
                        name: Name::from("parent"),
                        parent_id: None,
                        state: YakState::Wip,
                        context: None,
                        fields: std::collections::HashMap::new(),
                        tags: vec![],
                        created_by: Author::unknown(),
                        created_at: Timestamp::zero(),
                    },
                    Yak {
                        // Stores now return leaf names with explicit parent_id
                        id: YakId::from("child-bbbb"),
                        name: Name::from("child"),
                        parent_id: Some(YakId::from("parent-aaaa")),
                        state: YakState::Todo,
                        context: None,
                        fields: std::collections::HashMap::new(),
                        tags: vec![],
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

            fn list_blockers(&self) -> Result<Vec<crate::domain::YakBlockerSnapshot>> {
                Ok(Vec::new())
            }
        }

        let map = YakMap::from_store(&MockStore, EventMetadata::default_legacy()).unwrap();
        let child = map.yaks.get(&YakId::from("child-bbbb")).unwrap();
        assert_eq!(child.name, Name::from("child"));
        assert_eq!(child.parent_id, Some(YakId::from("parent-aaaa")));
    }

    #[test]
    fn test_from_events_replays_events_correctly() {
        use crate::domain::event_metadata::EventMetadata;
        use crate::domain::events::*;
        use crate::domain::YakEvent;

        let metadata = EventMetadata::default_legacy();

        // Create a sequence of events
        let events = vec![
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-aaaa"),
                    parent_id: None,
                },
                metadata.clone(),
            ),
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("parent-aaaa"),
                    field_name: ".state".to_string(),
                    content: "wip".to_string(),
                },
                metadata.clone(),
            ),
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-bbbb"),
                    parent_id: Some(YakId::from("parent-aaaa")),
                },
                metadata.clone(),
            ),
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("child-bbbb"),
                    field_name: ".context.md".to_string(),
                    content: "Test context".to_string(),
                },
                metadata.clone(),
            ),
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("child-bbbb"),
                    field_name: ".tags".to_string(),
                    content: "tag1\ntag2".to_string(),
                },
                metadata.clone(),
            ),
        ];

        let map = YakMap::from_events(events, metadata).unwrap();

        // Verify parent
        let parent = map.yaks.get(&YakId::from("parent-aaaa")).unwrap();
        assert_eq!(parent.name, Name::from("parent"));
        assert_eq!(parent.state, YakState::Wip);
        assert_eq!(parent.parent_id, None);

        // Verify child
        let child = map.yaks.get(&YakId::from("child-bbbb")).unwrap();
        assert_eq!(child.name, Name::from("child"));
        assert_eq!(child.state, YakState::Todo);
        assert_eq!(child.parent_id, Some(YakId::from("parent-aaaa")));
        assert_eq!(child.context, Some("Test context".to_string()));
        assert_eq!(child.tags, vec!["tag1", "tag2"]);
    }

    #[test]
    fn test_from_events_handles_compacted_event() {
        use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
        use crate::domain::YakEvent;

        let metadata = EventMetadata::default_legacy();

        // Create a compacted event with snapshots
        let snapshots = vec![
            Yak {
                id: YakId::from("yak1-aaaa"),
                name: Name::from("yak1"),
                parent_id: None,
                state: YakState::Wip,
                context: Some("context1".to_string()),
                fields: HashMap::new(),
                tags: vec!["tag1".to_string()],
                created_by: Author::unknown(),
                created_at: Timestamp::zero(),
            },
            Yak {
                id: YakId::from("yak2-bbbb"),
                name: Name::from("yak2"),
                parent_id: Some(YakId::from("yak1-aaaa")),
                state: YakState::Done,
                context: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: Author::unknown(),
                created_at: Timestamp::zero(),
            },
        ];

        let events = vec![YakEvent::Compacted(
            crate::domain::YakMapSnapshot::legacy(snapshots, vec![]),
            metadata.clone(),
        )];

        let map = YakMap::from_events(events, metadata).unwrap();

        // Verify both yaks were loaded from snapshot
        assert_eq!(map.yaks.len(), 2);
        let yak1 = map.yaks.get(&YakId::from("yak1-aaaa")).unwrap();
        assert_eq!(yak1.name, Name::from("yak1"));
        assert_eq!(yak1.state, YakState::Wip);
        assert_eq!(yak1.context, Some("context1".to_string()));
        assert_eq!(yak1.tags, vec!["tag1"]);

        let yak2 = map.yaks.get(&YakId::from("yak2-bbbb")).unwrap();
        assert_eq!(yak2.name, Name::from("yak2"));
        assert_eq!(yak2.state, YakState::Done);
        assert_eq!(yak2.parent_id, Some(YakId::from("yak1-aaaa")));
    }

    #[test]
    fn test_from_events_handles_removed_event() {
        use crate::domain::event_metadata::EventMetadata;
        use crate::domain::events::*;
        use crate::domain::YakEvent;

        let metadata = EventMetadata::default_legacy();

        let events = vec![
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("yak1"),
                    id: YakId::from("yak1-aaaa"),
                    parent_id: None,
                },
                metadata.clone(),
            ),
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("yak2"),
                    id: YakId::from("yak2-bbbb"),
                    parent_id: None,
                },
                metadata.clone(),
            ),
            YakEvent::Removed(
                RemovedEvent {
                    id: YakId::from("yak1-aaaa"),
                },
                metadata.clone(),
            ),
        ];

        let map = YakMap::from_events(events, metadata).unwrap();

        // Verify only yak2 remains
        assert_eq!(map.yaks.len(), 1);
        assert!(!map.yaks.contains_key(&YakId::from("yak1-aaaa")));
        assert!(map.yaks.contains_key(&YakId::from("yak2-bbbb")));
    }

    #[test]
    fn test_from_events_handles_moved_event() {
        use crate::domain::event_metadata::EventMetadata;
        use crate::domain::events::*;
        use crate::domain::YakEvent;

        let metadata = EventMetadata::default_legacy();

        let events = vec![
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent1"),
                    id: YakId::from("parent1-aaaa"),
                    parent_id: None,
                },
                metadata.clone(),
            ),
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent2"),
                    id: YakId::from("parent2-bbbb"),
                    parent_id: None,
                },
                metadata.clone(),
            ),
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-cccc"),
                    parent_id: Some(YakId::from("parent1-aaaa")),
                },
                metadata.clone(),
            ),
            YakEvent::Moved(
                MovedEvent {
                    id: YakId::from("child-cccc"),
                    new_parent: Some(YakId::from("parent2-bbbb")),
                },
                metadata.clone(),
            ),
        ];

        let map = YakMap::from_events(events, metadata).unwrap();

        // Verify child moved to parent2
        let child = map.yaks.get(&YakId::from("child-cccc")).unwrap();
        assert_eq!(child.parent_id, Some(YakId::from("parent2-bbbb")));
    }

    #[test]
    fn test_from_events_handles_name_change() {
        use crate::domain::event_metadata::EventMetadata;
        use crate::domain::events::*;
        use crate::domain::YakEvent;

        let metadata = EventMetadata::default_legacy();

        let events = vec![
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("old name"),
                    id: YakId::from("yak-aaaa"),
                    parent_id: None,
                },
                metadata.clone(),
            ),
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("yak-aaaa"),
                    field_name: ".name".to_string(),
                    content: "new name".to_string(),
                },
                metadata.clone(),
            ),
        ];

        let map = YakMap::from_events(events, metadata).unwrap();

        let yak = map.yaks.get(&YakId::from("yak-aaaa")).unwrap();
        assert_eq!(yak.name, Name::from("new name"));
    }

    #[test]
    fn test_from_events_handles_custom_fields() {
        use crate::domain::event_metadata::EventMetadata;
        use crate::domain::events::*;
        use crate::domain::YakEvent;

        let metadata = EventMetadata::default_legacy();

        let events = vec![
            YakEvent::Added(
                AddedEvent {
                    name: Name::from("yak"),
                    id: YakId::from("yak-aaaa"),
                    parent_id: None,
                },
                metadata.clone(),
            ),
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("yak-aaaa"),
                    field_name: "notes".to_string(),
                    content: "some notes".to_string(),
                },
                metadata.clone(),
            ),
            YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("yak-aaaa"),
                    field_name: "plan".to_string(),
                    content: "some plan".to_string(),
                },
                metadata.clone(),
            ),
        ];

        let map = YakMap::from_events(events, metadata).unwrap();

        let yak = map.yaks.get(&YakId::from("yak-aaaa")).unwrap();
        assert_eq!(yak.fields.get("notes"), Some(&"some notes".to_string()));
        assert_eq!(yak.fields.get("plan"), Some(&"some plan".to_string()));
    }

    #[test]
    fn test_from_store_uses_parent_id_from_yak() {
        use crate::domain::ports::ReadYakStore;
        use crate::domain::Yak;

        struct MockStore;

        impl ReadYakStore for MockStore {
            fn get_yak(&self, _id: &YakId) -> Result<Yak> {
                anyhow::bail!("Not needed")
            }

            fn list_yaks(&self) -> Result<Vec<Yak>> {
                use crate::domain::event_metadata::{Author, Timestamp};
                Ok(vec![
                    Yak {
                        id: YakId::from("parent-aaaa"),
                        name: Name::from("parent"),
                        parent_id: None,
                        state: YakState::Wip,
                        context: None,
                        fields: std::collections::HashMap::new(),
                        tags: vec![],
                        created_by: Author::unknown(),
                        created_at: Timestamp::zero(),
                    },
                    Yak {
                        id: YakId::from("child-bbbb"),
                        name: Name::from("child"),
                        parent_id: Some(YakId::from("parent-aaaa")),
                        state: YakState::Todo,
                        context: None,
                        fields: std::collections::HashMap::new(),
                        tags: vec![],
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

            fn list_blockers(&self) -> Result<Vec<crate::domain::YakBlockerSnapshot>> {
                Ok(Vec::new())
            }
        }

        let map = YakMap::from_store(&MockStore, EventMetadata::default_legacy()).unwrap();
        let child = map.yaks.get(&YakId::from("child-bbbb")).unwrap();
        assert_eq!(child.name, Name::from("child"));
        assert_eq!(
            child.parent_id,
            Some(YakId::from("parent-aaaa")),
            "from_store should use parent_id from Yak struct"
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
        assert_eq!(map.yaks.get(&id).unwrap().state, YakState::Todo);
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
    fn test_add_yak_stores_name_in_yak_entry() {
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

        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Todo);
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

        assert_eq!(map.yaks.get(&a_id).unwrap().state, YakState::Todo);
        assert_eq!(map.yaks.get(&b_id).unwrap().state, YakState::Todo);
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

        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Todo);
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
        assert_eq!(map.yaks.get(&id).unwrap().state, YakState::Wip);
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
    fn test_update_state_prevents_marking_explicitly_blocked_yak_done_without_event() {
        let mut map = YakMap::new();
        let target_id = map
            .add_yak("blocked yak", None, None, None, None, vec![])
            .unwrap();
        let blocker_id = map
            .add_yak("blocking yak", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target_id.clone(), blocker_id, None)
            .unwrap();
        map.take_events();

        let result = map.update_state(target_id.clone(), "done".to_string());

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot mark 'blocked yak' as done"));
        assert!(err.contains("blocked by blocking yak"));
        assert_eq!(map.yaks.get(&target_id).unwrap().state, YakState::Todo);
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn test_is_ready_false_for_todo_parent_with_incomplete_child() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        map.add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();

        assert!(!map.is_ready(&parent_id).unwrap());
    }

    #[test]
    fn test_is_ready_true_for_todo_parent_with_all_direct_children_done() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_1_id = map
            .add_yak("child 1", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        let child_2_id = map
            .add_yak("child 2", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        map.update_state(child_1_id, "done".to_string()).unwrap();
        map.update_state(child_2_id, "done".to_string()).unwrap();

        assert!(map.is_ready(&parent_id).unwrap());
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Todo);
    }

    #[test]
    fn test_is_ready_false_when_state_is_not_todo() {
        let mut map = YakMap::new();
        let id = map.add_yak("yak", None, None, None, None, vec![]).unwrap();
        map.update_state(id.clone(), "wip".to_string()).unwrap();

        assert!(!map.is_ready(&id).unwrap());
    }

    #[test]
    fn test_update_state_does_not_promote_parent_on_todo_transition() {
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

        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Todo);
        assert_eq!(map.yaks.get(&child_id).unwrap().state, YakState::Wip);
        let events = map.take_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::FieldUpdated(event, _) => assert_eq!(event.id, child_id),
            _ => panic!("Expected child state event"),
        }
    }

    #[test]
    fn test_update_state_does_not_promote_ancestors_through_multiple_levels() {
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

        assert_eq!(map.yaks.get(&a_id).unwrap().state, YakState::Todo);
        assert_eq!(map.yaks.get(&b_id).unwrap().state, YakState::Todo);
        assert_eq!(map.yaks.get(&c_id).unwrap().state, YakState::Wip);
        assert_eq!(map.take_events().len(), 1);
    }

    #[test]
    fn test_update_state_wip_to_done_emits_only_child_event() {
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
    fn test_update_state_done_child_to_done_does_not_demote_done_parent() {
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

        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();

        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Done);
        assert_eq!(map.yaks.get(&child_id).unwrap().state, YakState::Done);

        let events = map.take_events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            YakEvent::FieldUpdated(event, _) => {
                assert_eq!(event.id, child_id);
                assert_eq!(event.field_name, ".state");
                assert_eq!(event.content, "done");
            }
            _ => panic!("Expected child state event"),
        }
    }

    #[test]
    fn test_update_state_demotes_done_parent_to_todo_when_child_leaves_done() {
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

        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Todo);
        assert_eq!(map.yaks.get(&child_id).unwrap().state, YakState::Wip);
    }

    #[test]
    fn test_update_state_demotes_done_ancestors_to_todo_through_multiple_levels() {
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

        assert_eq!(map.yaks.get(&a_id).unwrap().state, YakState::Todo);
        assert_eq!(map.yaks.get(&b_id).unwrap().state, YakState::Todo);
        assert_eq!(map.yaks.get(&c_id).unwrap().state, YakState::Wip);
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
        // parent is todo (not auto-promoted), not done
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Todo);
        map.take_events();
        map.update_state(child_id.clone(), "wip".to_string())
            .unwrap();
        // parent stays todo, not affected
        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Todo);
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

    #[test]
    fn done_removes_explicit_relationships_where_yak_is_blocker_before_state_change() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocker", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        map.update_state(blocker.clone(), "done".to_string())
            .unwrap();
        let events = map.take_events();

        assert!(map.active_blockers(&target).is_empty());
        assert!(matches!(
            &events[..],
            [
                YakEvent::BlockerRemoved(BlockerRemovedEvent { target: removed_target, source: BlockerSource::Yak(removed_blocker) }, _),
                YakEvent::FieldUpdated(FieldUpdatedEvent { id, field_name, content }, _)
            ] if removed_target == &target
                && removed_blocker == &blocker
                && id == &blocker
                && field_name == ".state"
                && content == "done"
        ));
    }

    #[test]
    fn remove_yak_emits_blocker_removed_for_relationships_where_yak_is_target() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocker", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        map.remove_yak(target.clone()).unwrap();
        let events = map.take_events();

        assert!(matches!(
            &events[..],
            [
                YakEvent::BlockerRemoved(BlockerRemovedEvent { target: removed_target, source: BlockerSource::Yak(removed_blocker) }, _),
                YakEvent::Removed(RemovedEvent { id }, _)
            ] if removed_target == &target && removed_blocker == &blocker && id == &target
        ));
    }

    #[test]
    fn remove_yak_emits_blocker_removed_for_relationships_where_yak_is_blocker() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocker", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        map.remove_yak(blocker.clone()).unwrap();
        let events = map.take_events();

        assert!(map.active_blockers(&target).is_empty());
        assert!(matches!(
            &events[..],
            [
                YakEvent::BlockerRemoved(BlockerRemovedEvent { target: removed_target, source: BlockerSource::Yak(removed_blocker) }, _),
                YakEvent::Removed(RemovedEvent { id }, _)
            ] if removed_target == &target && removed_blocker == &blocker && id == &blocker
        ));
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

    #[test]
    fn ancestor_ids_after_move_uses_proposed_parent_for_moved_yak() {
        let mut map = YakMap::new();
        let old_parent = map.add_yak("old", None, None, None, None, vec![]).unwrap();
        let moved = map
            .add_yak("moved", Some(old_parent), None, None, None, vec![])
            .unwrap();
        let new_parent = map.add_yak("new", None, None, None, None, vec![]).unwrap();

        assert_eq!(
            map.ancestor_ids_after_move(&moved, &moved, &Some(new_parent.clone())),
            vec![new_parent]
        );
    }

    #[test]
    fn ancestor_ids_after_move_stops_at_cycle_in_proposed_parent_chain() {
        let mut map = YakMap::new();
        let moved = map
            .add_yak("moved", None, None, None, None, vec![])
            .unwrap();

        assert_eq!(
            map.ancestor_ids_after_move(&moved, &moved, &Some(moved.clone())),
            vec![moved]
        );
    }

    #[test]
    fn move_blocker_under_blocked_yak_removes_redundant_explicit_blocker() {
        let mut map = YakMap::new();
        let target = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let blocker = map.add_yak("b", None, None, None, None, vec![]).unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        map.move_yak_to(blocker.clone(), Some(target.clone()))
            .unwrap();
        let events = map.take_events();

        assert!(map.active_blockers(&target).is_empty());
        assert_eq!(
            map.yaks.get(&blocker).unwrap().parent_id,
            Some(target.clone())
        );
        assert!(matches!(
            &events[..],
            [
                YakEvent::BlockerRemoved(BlockerRemovedEvent { target: removed_target, source: BlockerSource::Yak(removed_blocker) }, _),
                YakEvent::Moved(MovedEvent { id, new_parent }, _)
            ] if removed_target == &target
                && removed_blocker == &blocker
                && id == &blocker
                && new_parent.as_ref() == Some(&target)
        ));
    }

    #[test]
    fn move_subtree_under_blocked_yak_removes_descendant_redundant_explicit_blocker() {
        let mut map = YakMap::new();
        let target = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let moved = map.add_yak("b", None, None, None, None, vec![]).unwrap();
        let descendant = map
            .add_yak("c", Some(moved.clone()), None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), descendant.clone(), None)
            .unwrap();
        map.take_events();

        map.move_yak_to(moved.clone(), Some(target.clone()))
            .unwrap();
        let events = map.take_events();

        assert!(map.active_blockers(&target).is_empty());
        assert_eq!(
            map.yaks.get(&moved).unwrap().parent_id,
            Some(target.clone())
        );
        assert!(matches!(
            &events[..],
            [
                YakEvent::BlockerRemoved(BlockerRemovedEvent { target: removed_target, source: BlockerSource::Yak(removed_blocker) }, _),
                YakEvent::Moved(MovedEvent { id, new_parent }, _)
            ] if removed_target == &target
                && removed_blocker == &descendant
                && id == &moved
                && new_parent.as_ref() == Some(&target)
        ));
    }

    #[test]
    fn move_subtree_under_descendant_blocker_is_rejected_as_circular_dependency() {
        let mut map = YakMap::new();
        let new_parent = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let moved = map.add_yak("b", None, None, None, None, vec![]).unwrap();
        let descendant = map
            .add_yak("c", Some(moved.clone()), None, None, None, vec![])
            .unwrap();
        map.add_blocker(descendant.clone(), new_parent.clone(), None)
            .unwrap();
        map.take_events();

        let err = map
            .move_yak_to(moved.clone(), Some(new_parent.clone()))
            .unwrap_err()
            .to_string();

        assert!(err.contains("would create circular dependency"));
        assert!(!err.contains("through hierarchy"));
        assert!(err.contains("a"));
        assert!(err.contains("b/c"));
        assert_eq!(map.yaks.get(&moved).unwrap().parent_id, None);
        assert!(map
            .active_blockers(&descendant)
            .iter()
            .any(|b| b.source == BlockerSource::Yak(new_parent.clone())));
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn move_blocked_yak_under_its_blocker_is_rejected_as_circular_dependency() {
        let mut map = YakMap::new();
        let target = map.add_yak("a", None, None, None, None, vec![]).unwrap();
        let blocker = map.add_yak("b", None, None, None, None, vec![]).unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        let err = map
            .move_yak_to(target.clone(), Some(blocker.clone()))
            .unwrap_err()
            .to_string();

        assert!(err.contains("would create circular dependency"));
        assert!(err.contains("a -> b -> a"));
        assert_eq!(map.yaks.get(&target).unwrap().parent_id, None);
        assert!(map
            .active_blockers(&target)
            .iter()
            .any(|b| b.source == BlockerSource::Yak(blocker.clone())));
        assert!(map.take_events().is_empty());
    }

    #[test]
    fn move_ancestor_under_explicit_blocker_is_rejected_as_circular_dependency() {
        let mut map = YakMap::new();
        let project = map
            .add_yak("project", None, None, None, None, vec![])
            .unwrap();
        let target = map
            .add_yak("a", Some(project.clone()), None, None, None, vec![])
            .unwrap();
        let blocker = map.add_yak("b", None, None, None, None, vec![]).unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        let err = map
            .move_yak_to(project.clone(), Some(blocker.clone()))
            .unwrap_err()
            .to_string();

        assert!(err.contains("would create circular dependency"));
        assert!(!err.contains("through hierarchy"));
        assert!(err.contains("project -> b -> project/a -> project"));
        assert_eq!(map.yaks.get(&project).unwrap().parent_id, None);
        assert!(map
            .active_blockers(&target)
            .iter()
            .any(|b| b.source == BlockerSource::Yak(blocker.clone())));
        assert!(map.take_events().is_empty());
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

        map.prune(None).unwrap();

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
        map.update_state(child_id.clone(), "done".to_string())
            .unwrap();
        map.update_state(parent_id.clone(), "done".to_string())
            .unwrap();
        map.take_events();

        map.prune(None).unwrap();

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

        map.prune(None).unwrap();
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
    fn prune_emits_blocker_removed_for_relationships_touching_pruned_yak() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocker", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.update_state(blocker.clone(), "done".to_string())
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        map.prune(None).unwrap();
        let events = map.take_events();

        assert!(map.active_blockers(&target).is_empty());
        assert!(matches!(
            &events[..],
            [
                YakEvent::BlockerRemoved(BlockerRemovedEvent { target: removed_target, source: BlockerSource::Yak(removed_blocker) }, _),
                YakEvent::Removed(RemovedEvent { id }, _)
            ] if removed_target == &target && removed_blocker == &blocker && id == &blocker
        ));
    }

    #[test]
    fn replaying_removed_event_clears_active_blockers_touching_removed_yak() {
        let mut map = YakMap::new();
        let target = map
            .add_yak("blocked", None, None, None, None, vec![])
            .unwrap();
        let blocker = map
            .add_yak("blocker", None, None, None, None, vec![])
            .unwrap();
        map.add_blocker(target.clone(), blocker.clone(), None)
            .unwrap();
        map.take_events();

        map.apply_removed(RemovedEvent { id: blocker });

        assert!(map.active_blockers(&target).is_empty());
    }

    // Tests for enriched add_yak parameters
    #[test]
    fn test_add_yak_with_initial_state() {
        let mut map = YakMap::new();

        let id = map
            .add_yak("test", None, None, Some("wip".to_string()), None, vec![])
            .unwrap();

        assert_eq!(map.yaks.get(&id).unwrap().state, YakState::Wip);

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

    // Tests for state transition conditions.

    #[test]
    fn test_wip_to_done_does_not_promote_todo_parent() {
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

        // Parent should remain Todo - propagation should NOT fire
        assert_eq!(
            map.yaks.get(&parent_id).unwrap().state,
            YakState::Todo,
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
    fn test_todo_to_wip_emits_no_parent_state_event() {
        let mut map = YakMap::new();
        let parent_id = map
            .add_yak("parent", None, None, None, None, vec![])
            .unwrap();
        let child_id = map
            .add_yak("child", Some(parent_id.clone()), None, None, None, vec![])
            .unwrap();
        map.take_events();

        map.update_state(child_id, "wip".to_string()).unwrap();

        assert_eq!(map.yaks.get(&parent_id).unwrap().state, YakState::Todo);
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
        assert_eq!(map.yaks.get(&id).unwrap().state, YakState::Wip);
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
