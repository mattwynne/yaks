use anyhow::Result;
use git2::Repository;
use std::path::Path;

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::ports::{EventStore, EventStoreReader};
use crate::domain::{Yak, YakEvent};

/// Builds a git tree object representing a single yak's subtree.
///
/// A yak subtree contains blobs for each field (state, context.md, name,
/// etc.) plus optional metadata. This builder provides a single place to
/// construct these subtrees, used by both `build_tree_from_event` (for
/// the `Added` event) and `reset_from_snapshot`.
///
/// # Example
///
/// ```ignore
/// let oid = YakSubtreeBuilder::new(&repo)
///     .name("fix the tests")
///     .state("todo")
///     .context("")
///     .metadata(&author, timestamp)
///     .parent_id(Some("parent-a1b2"))
///     .build()?;
/// ```
struct YakSubtreeBuilder<'r> {
    repo: &'r Repository,
    entries: Vec<(&'static str, String)>,
    custom_fields: Vec<(String, String)>,
}

impl<'r> YakSubtreeBuilder<'r> {
    fn new(repo: &'r Repository) -> Self {
        Self {
            repo,
            entries: Vec::new(),
            custom_fields: Vec::new(),
        }
    }

    /// Set the yak's display name.
    fn name(mut self, name: &str) -> Self {
        self.entries.push(("name", name.to_string()));
        self
    }

    /// Set the yak's state (todo, wip, done).
    fn state(mut self, state: &str) -> Self {
        self.entries.push(("state", state.to_string()));
        self
    }

    /// Set the yak's context markdown content.
    fn context(mut self, content: &str) -> Self {
        self.entries.push(("context.md", content.to_string()));
        self
    }

    /// Set the yak's ID blob (used by `reset_from_snapshot`).
    fn id(mut self, id: &str) -> Self {
        self.entries.push(("id", id.to_string()));
        self
    }

    /// Set the parent yak's ID, if this yak is nested.
    fn parent_id(mut self, parent_id: Option<&str>) -> Self {
        if let Some(pid) = parent_id {
            self.entries.push(("parent_id", pid.to_string()));
        }
        self
    }

    /// Write the `.metadata.json` blob with author and timestamp.
    fn metadata(mut self, author: &Author, timestamp: Timestamp) -> Self {
        let json = serde_json::json!({
            "created_by": {
                "name": author.name,
                "email": author.email
            },
            "created_at": timestamp.as_epoch_secs()
        });
        self.entries.push((".metadata.json", json.to_string()));
        self
    }

    /// Add custom (non-reserved) fields to the subtree.
    fn custom_fields(mut self, fields: &std::collections::HashMap<String, String>) -> Self {
        for (name, content) in fields {
            self.custom_fields.push((name.clone(), content.clone()));
        }
        self
    }

    /// Write all collected entries to a new git tree object.
    fn build(self) -> Result<git2::Oid> {
        let mut builder = self.repo.treebuilder(None)?;

        for (name, content) in &self.entries {
            let blob = self.repo.blob(content.as_bytes())?;
            builder.insert(name, blob, 0o100644)?;
        }

        for (name, content) in &self.custom_fields {
            let blob = self.repo.blob(content.as_bytes())?;
            builder.insert(name, blob, 0o100644)?;
        }

        Ok(builder.write()?)
    }
}

pub struct GitEventStore {
    repo: Repository,
    ref_name: String,
}

impl GitEventStore {
    pub fn new(repo_path: &Path) -> Result<Self> {
        let repo = Repository::open(repo_path)
            .map_err(|_| anyhow::anyhow!("Error: not in a git repository"))?;
        Ok(Self {
            repo,
            ref_name: "refs/notes/yaks".to_string(),
        })
    }

    /// Create a GitEventStore that reads/writes a custom ref name.
    pub fn with_ref_name(repo_path: &Path, ref_name: &str) -> Result<Self> {
        let repo = Repository::open(repo_path)
            .map_err(|_| anyhow::anyhow!("Error: not in a git repository"))?;
        Ok(Self {
            repo,
            ref_name: ref_name.to_string(),
        })
    }

    /// For tests: create from an already-opened Repository
    #[cfg(test)]
    pub fn from_repo(repo: Repository) -> Self {
        Self {
            repo,
            ref_name: "refs/notes/yaks".to_string(),
        }
    }

    /// Get the latest commit on refs/notes/yaks, if any
    fn get_latest_commit(&self) -> Result<Option<git2::Commit<'_>>> {
        match self.repo.refname_to_id(&self.ref_name) {
            Ok(oid) => Ok(Some(self.repo.find_commit(oid)?)),
            Err(_) => Ok(None),
        }
    }

    /// Get the current tree from refs/notes/yaks, if any
    fn get_current_tree(&self) -> Result<Option<git2::Tree<'_>>> {
        match self.get_latest_commit()? {
            Some(commit) => Ok(Some(commit.tree()?)),
            None => Ok(None),
        }
    }

    /// Get a yak's subtree from the root tree by its ID (direct root lookup).
    fn get_yak_subtree(
        &self,
        root: Option<&git2::Tree>,
        yak_id: &str,
    ) -> Result<Option<git2::Tree<'_>>> {
        let Some(root) = root else {
            return Ok(None);
        };

        match root.get_name(yak_id) {
            Some(entry) => Ok(Some(self.repo.find_tree(entry.id())?)),
            None => Ok(None),
        }
    }

    /// Update a file in a yak's subtree, returning new root tree OID
    fn update_yak_file(
        &self,
        current_tree: Option<&git2::Tree>,
        yak_id: &str,
        file_name: &str,
        content: &str,
    ) -> Result<git2::Oid> {
        let blob_oid = self.repo.blob(content.as_bytes())?;

        // Build the yak's subtree
        let yak_subtree = self.get_yak_subtree(current_tree, yak_id)?;
        let mut yak_builder = self.repo.treebuilder(yak_subtree.as_ref())?;
        yak_builder.insert(file_name, blob_oid, 0o100644)?;
        let yak_tree_oid = yak_builder.write()?;

        // Rebuild root tree with updated yak subtree
        self.set_yak_in_root(current_tree, yak_id, Some(yak_tree_oid))
    }

    /// Set (or remove) a yak subtree in the root tree.
    fn set_yak_in_root(
        &self,
        root: Option<&git2::Tree>,
        yak_id: &str,
        subtree_oid: Option<git2::Oid>,
    ) -> Result<git2::Oid> {
        let mut builder = self.repo.treebuilder(root)?;
        match subtree_oid {
            Some(oid) => {
                builder.insert(yak_id, oid, 0o040000)?;
            }
            None => {
                let _ = builder.remove(yak_id);
            }
        }
        Ok(builder.write()?)
    }

    /// Build an updated tree by applying an event to the current tree.
    /// All operations happen in git's object database - no filesystem IO.
    fn build_tree_from_event(
        &self,
        event: &YakEvent,
        current_tree: Option<&git2::Tree>,
    ) -> Result<git2::Oid> {
        match event {
            YakEvent::Added(e, metadata) => {
                let yak_tree_oid = YakSubtreeBuilder::new(&self.repo)
                    .name(e.name.as_str())
                    .state("todo")
                    .context("")
                    .metadata(&metadata.author, metadata.timestamp)
                    .parent_id(e.parent_id.as_ref().map(|p| p.as_str()))
                    .build()?;
                self.set_yak_in_root(current_tree, e.id.as_str(), Some(yak_tree_oid))
            }

            YakEvent::Removed(e, _) => {
                // Flat: yak is always at root by its ID
                self.set_yak_in_root(current_tree, e.id.as_str(), None)
            }

            YakEvent::Moved(e, _) => {
                // In flat structure, moving just updates the parent_id blob
                let yak_id = e.id.as_str();
                let subtree = self.get_yak_subtree(current_tree, yak_id)?;
                let mut builder = self.repo.treebuilder(subtree.as_ref())?;

                match &e.new_parent {
                    Some(parent_id) => {
                        let blob = self.repo.blob(parent_id.as_str().as_bytes())?;
                        builder.insert("parent_id", blob, 0o100644)?;
                    }
                    None => {
                        let _ = builder.remove("parent_id");
                    }
                }

                let new_subtree_oid = builder.write()?;
                self.set_yak_in_root(current_tree, yak_id, Some(new_subtree_oid))
            }

            YakEvent::FieldUpdated(e, _) => {
                // Flat: yak is always at root by its ID
                self.update_yak_file(current_tree, e.id.as_str(), &e.field_name, &e.content)
            }

            YakEvent::Compacted(_) => {
                // Compacted keeps the current tree unchanged — the
                // snapshot IS the current tree state.
                match current_tree {
                    Some(tree) => Ok(tree.id()),
                    None => {
                        let builder = self.repo.treebuilder(None)?;
                        Ok(builder.write()?)
                    }
                }
            }
        }
    }
    /// Check if any existing commit has the given Event-Id trailer
    fn has_event_id(&self, event_id: &str) -> Result<bool> {
        let Some(latest) = self.get_latest_commit()? else {
            return Ok(false);
        };

        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push(latest.id())?;

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let message = commit.message().unwrap_or("");
            if Self::message_has_event_id(message, event_id) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if a commit message contains a specific Event-Id trailer
    fn message_has_event_id(message: &str, event_id: &str) -> bool {
        let prefix = "Event-Id: ";
        for line in message.lines() {
            let trimmed = line.trim();
            if let Some(id) = trimmed.strip_prefix(prefix) {
                if id.trim() == event_id {
                    return true;
                }
            }
        }
        false
    }

    /// Extract the Event-Id from a commit message, falling back to a
    /// provided default (typically the commit SHA for legacy commits).
    fn extract_event_id(message: &str, fallback: &str) -> String {
        let prefix = "Event-Id: ";
        for line in message.lines() {
            let trimmed = line.trim();
            if let Some(id) = trimmed.strip_prefix(prefix) {
                let id = id.trim();
                if !id.is_empty() {
                    return id.to_string();
                }
            }
        }
        fallback.to_string()
    }

    /// Synthesize domain events from a compaction tree, preserving
    /// the existing yak IDs stored as tree entry names.
    #[allow(clippy::cognitive_complexity)]
    fn collect_compaction_events(
        &self,
        tree: &git2::Tree,
        events: &mut Vec<YakEvent>,
    ) -> Result<()> {
        use crate::domain::field::RESERVED_FIELDS;
        use crate::domain::slug::{Name, YakId};
        use std::collections::HashSet;

        // Collect yak data from root-level tree entries
        struct YakData {
            id: String,
            name_str: String,
            subtree_id: git2::Oid,
            parent_id_str: Option<String>,
        }

        let mut yak_data: Vec<YakData> = Vec::new();

        for entry in tree.iter() {
            if entry.kind() != Some(git2::ObjectType::Tree) {
                continue;
            }
            let entry_name = match entry.name() {
                Some(n) => n.to_string(),
                None => continue,
            };

            let subtree = self.repo.find_tree(entry.id())?;

            let is_yak =
                subtree.get_name("state").is_some() || subtree.get_name("context.md").is_some();
            if !is_yak {
                continue;
            }

            let name_str = if let Some(name_entry) = subtree.get_name("name") {
                let name_blob = self.repo.find_blob(name_entry.id())?;
                std::str::from_utf8(name_blob.content())?.trim().to_string()
            } else {
                entry_name.clone()
            };

            let parent_id_str = if let Some(pid_entry) = subtree.get_name("parent_id") {
                let pid_blob = self.repo.find_blob(pid_entry.id())?;
                Some(std::str::from_utf8(pid_blob.content())?.trim().to_string())
            } else {
                None
            };

            yak_data.push(YakData {
                id: entry_name,
                name_str,
                subtree_id: entry.id(),
                parent_id_str,
            });
        }

        // Topological sort: parents before children
        let mut emitted: HashSet<String> = HashSet::new();
        let mut remaining = yak_data;
        let mut ordered: Vec<YakData> = Vec::new();

        loop {
            let before = remaining.len();
            let mut still_remaining = Vec::new();

            for item in remaining {
                let can_emit = match &item.parent_id_str {
                    None => true,
                    Some(pid) => emitted.contains(pid),
                };
                if can_emit {
                    emitted.insert(item.id.clone());
                    ordered.push(item);
                } else {
                    still_remaining.push(item);
                }
            }

            remaining = still_remaining;
            if remaining.is_empty() || remaining.len() == before {
                ordered.extend(remaining);
                break;
            }
        }

        // Emit events using preserved IDs
        for data in &ordered {
            let id = YakId::from(data.id.as_str());
            let name = Name::from(data.name_str.as_str());
            let parent_id = data.parent_id_str.as_ref().map(|p| YakId::from(p.as_str()));

            let subtree = self.repo.find_tree(data.subtree_id)?;

            // Read .metadata.json if present
            let added_metadata = if let Some(meta_entry) = subtree.get_name(".metadata.json") {
                if let Ok(meta_blob) = self.repo.find_blob(meta_entry.id()) {
                    if let Ok(content) = std::str::from_utf8(meta_blob.content()) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                            use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
                            EventMetadata::new(
                                Author {
                                    name: json["created_by"]["name"]
                                        .as_str()
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    email: json["created_by"]["email"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                },
                                Timestamp(json["created_at"].as_i64().unwrap_or(0)),
                            )
                        } else {
                            crate::domain::event_metadata::EventMetadata::default_legacy()
                        }
                    } else {
                        crate::domain::event_metadata::EventMetadata::default_legacy()
                    }
                } else {
                    crate::domain::event_metadata::EventMetadata::default_legacy()
                }
            } else {
                crate::domain::event_metadata::EventMetadata::default_legacy()
            };

            events.push(YakEvent::Added(
                crate::domain::events::AddedEvent {
                    name: name.clone(),
                    id: id.clone(),
                    parent_id: parent_id.clone(),
                },
                added_metadata,
            ));

            // State
            if let Some(state_entry) = subtree.get_name("state") {
                let state_blob = self.repo.find_blob(state_entry.id())?;
                let state = std::str::from_utf8(state_blob.content())?.trim();
                if state != "todo" {
                    events.push(YakEvent::FieldUpdated(
                        crate::domain::events::FieldUpdatedEvent {
                            id: id.clone(),
                            field_name: "state".to_string(),
                            content: state.to_string(),
                        },
                        crate::domain::event_metadata::EventMetadata::default_legacy(),
                    ));
                }
            }

            // Context
            if let Some(context_entry) = subtree.get_name("context.md") {
                let context_blob = self.repo.find_blob(context_entry.id())?;
                let content = std::str::from_utf8(context_blob.content())?;
                if !content.is_empty() {
                    events.push(YakEvent::FieldUpdated(
                        crate::domain::events::FieldUpdatedEvent {
                            id: id.clone(),
                            field_name: "context.md".to_string(),
                            content: content.to_string(),
                        },
                        crate::domain::event_metadata::EventMetadata::default_legacy(),
                    ));
                }
            }

            // Custom fields
            for field_entry in subtree.iter() {
                if field_entry.kind() != Some(git2::ObjectType::Blob) {
                    continue;
                }
                let field_name = match field_entry.name() {
                    Some(n) => n,
                    None => continue,
                };
                if RESERVED_FIELDS.contains(&field_name) {
                    continue;
                }
                let field_blob = self.repo.find_blob(field_entry.id())?;
                let content = std::str::from_utf8(field_blob.content())?;
                events.push(YakEvent::FieldUpdated(
                    crate::domain::events::FieldUpdatedEvent {
                        id: id.clone(),
                        field_name: field_name.to_string(),
                        content: content.to_string(),
                    },
                    crate::domain::event_metadata::EventMetadata::default_legacy(),
                ));
            }
        }

        Ok(())
    }

    /// Read the current git tree state and synthesize domain events.
    /// All yak IDs are regenerated using `generate_id(name, parent_id)`,
    /// making this suitable for repairing inconsistent data.
    pub fn snapshot_events(&self) -> Result<Vec<YakEvent>> {
        let tree = self.get_current_tree()?;
        let Some(tree) = tree else {
            return Ok(Vec::new());
        };

        let mut events = Vec::new();
        self.collect_snapshot_events(&tree, &mut events)?;
        Ok(events)
    }

    #[allow(clippy::cognitive_complexity)]
    fn collect_snapshot_events(&self, tree: &git2::Tree, events: &mut Vec<YakEvent>) -> Result<()> {
        use crate::domain::field::RESERVED_FIELDS;
        use crate::domain::slug::{generate_id, Name, YakId};
        use std::collections::{HashMap, HashSet};

        // First pass: collect all yak data from root-level entries
        struct YakData {
            name_str: String,
            subtree_id: git2::Oid,
            parent_id_str: Option<String>,
        }

        let mut yak_data: Vec<(String, YakData)> = Vec::new();

        for entry in tree.iter() {
            if entry.kind() != Some(git2::ObjectType::Tree) {
                continue;
            }
            let entry_name = match entry.name() {
                Some(n) => n.to_string(),
                None => continue,
            };

            let subtree = self.repo.find_tree(entry.id())?;

            // A yak subtree has a `state` or `context.md` blob
            let is_yak =
                subtree.get_name("state").is_some() || subtree.get_name("context.md").is_some();
            if !is_yak {
                continue;
            }

            // Read name from `name` blob, falling back to directory entry name
            let name_str = if let Some(name_entry) = subtree.get_name("name") {
                let name_blob = self.repo.find_blob(name_entry.id())?;
                std::str::from_utf8(name_blob.content())?.trim().to_string()
            } else {
                entry_name.clone()
            };

            // Read parent_id from blob if present
            let parent_id_str = if let Some(pid_entry) = subtree.get_name("parent_id") {
                let pid_blob = self.repo.find_blob(pid_entry.id())?;
                Some(std::str::from_utf8(pid_blob.content())?.trim().to_string())
            } else {
                None
            };

            yak_data.push((
                entry_name,
                YakData {
                    name_str,
                    subtree_id: entry.id(),
                    parent_id_str,
                },
            ));
        }

        // Topological sort: emit parentless yaks first, then yaks whose
        // parent has already been emitted.
        // Build a map from old tree-entry ID to parent_id for ordering.
        let mut emitted: HashSet<String> = HashSet::new();
        let mut remaining = yak_data;
        let mut ordered: Vec<(String, YakData)> = Vec::new();

        loop {
            let before = remaining.len();
            let mut still_remaining = Vec::new();

            for item in remaining {
                let can_emit = match &item.1.parent_id_str {
                    None => true,
                    Some(pid) => emitted.contains(pid),
                };
                if can_emit {
                    emitted.insert(item.0.clone());
                    ordered.push(item);
                } else {
                    still_remaining.push(item);
                }
            }

            remaining = still_remaining;
            if remaining.is_empty() || remaining.len() == before {
                // Append any remaining (orphans) at end
                ordered.extend(remaining);
                break;
            }
        }

        // Second pass: generate IDs and emit events.
        // We need to map old parent_id strings to regenerated YakIds.
        let mut old_entry_to_new_id: HashMap<String, YakId> = HashMap::new();
        let mut emitted_ids: HashSet<YakId> = HashSet::new();

        for (entry_name, data) in &ordered {
            let parent_yak_id: Option<YakId> = data
                .parent_id_str
                .as_ref()
                .and_then(|pid| old_entry_to_new_id.get(pid))
                .cloned();

            let id = generate_id(&data.name_str, parent_yak_id.as_ref());
            let name = Name::from(data.name_str.as_str());

            old_entry_to_new_id.insert(entry_name.clone(), id.clone());

            // Skip duplicate entries that resolve to the same yak ID.
            // This handles corrupted trees where multiple directory entries
            // (e.g. "config" and "config-7bvf") contain the same yak name.
            if !emitted_ids.insert(id.clone()) {
                continue;
            }

            let subtree = self.repo.find_tree(data.subtree_id)?;

            // Read .metadata.json if present
            let added_metadata = if let Some(meta_entry) = subtree.get_name(".metadata.json") {
                if let Ok(meta_blob) = self.repo.find_blob(meta_entry.id()) {
                    if let Ok(content) = std::str::from_utf8(meta_blob.content()) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
                            use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
                            EventMetadata::new(
                                Author {
                                    name: json["created_by"]["name"]
                                        .as_str()
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    email: json["created_by"]["email"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                },
                                Timestamp(json["created_at"].as_i64().unwrap_or(0)),
                            )
                        } else {
                            crate::domain::event_metadata::EventMetadata::default_legacy()
                        }
                    } else {
                        crate::domain::event_metadata::EventMetadata::default_legacy()
                    }
                } else {
                    crate::domain::event_metadata::EventMetadata::default_legacy()
                }
            } else {
                crate::domain::event_metadata::EventMetadata::default_legacy()
            };

            // Added event
            events.push(YakEvent::Added(
                crate::domain::events::AddedEvent {
                    name: name.clone(),
                    id: id.clone(),
                    parent_id: parent_yak_id.clone(),
                },
                added_metadata,
            ));

            // State
            if let Some(state_entry) = subtree.get_name("state") {
                let state_blob = self.repo.find_blob(state_entry.id())?;
                let state = std::str::from_utf8(state_blob.content())?.trim();
                if state != "todo" {
                    events.push(YakEvent::FieldUpdated(
                        crate::domain::events::FieldUpdatedEvent {
                            id: id.clone(),
                            field_name: "state".to_string(),
                            content: state.to_string(),
                        },
                        crate::domain::event_metadata::EventMetadata::default_legacy(),
                    ));
                }
            }

            // Context
            if let Some(context_entry) = subtree.get_name("context.md") {
                let context_blob = self.repo.find_blob(context_entry.id())?;
                let content = std::str::from_utf8(context_blob.content())?;
                if !content.is_empty() {
                    events.push(YakEvent::FieldUpdated(
                        crate::domain::events::FieldUpdatedEvent {
                            id: id.clone(),
                            field_name: "context.md".to_string(),
                            content: content.to_string(),
                        },
                        crate::domain::event_metadata::EventMetadata::default_legacy(),
                    ));
                }
            }

            // Custom fields
            for field_entry in subtree.iter() {
                if field_entry.kind() != Some(git2::ObjectType::Blob) {
                    continue;
                }
                let field_name = match field_entry.name() {
                    Some(n) => n,
                    None => continue,
                };
                if RESERVED_FIELDS.contains(&field_name) {
                    continue;
                }
                let field_blob = self.repo.find_blob(field_entry.id())?;
                let content = std::str::from_utf8(field_blob.content())?;
                events.push(YakEvent::FieldUpdated(
                    crate::domain::events::FieldUpdatedEvent {
                        id: id.clone(),
                        field_name: field_name.to_string(),
                        content: content.to_string(),
                    },
                    crate::domain::event_metadata::EventMetadata::default_legacy(),
                ));
            }
        }

        Ok(())
    }
}

impl EventStore for GitEventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()> {
        let event = super::ensure_event_id(event.clone());
        let event_id = event.metadata().event_id.clone().unwrap();

        // Idempotent: skip if we already have a commit with this event_id
        if self.has_event_id(&event_id)? {
            return Ok(());
        }

        let current_tree = self.get_current_tree()?;

        let tree_oid = self.build_tree_from_event(&event, current_tree.as_ref())?;
        let tree = self.repo.find_tree(tree_oid)?;

        // Commit message includes the event_id as a trailer for
        // stable cross-repo identity during sync.
        let event_line = event.format_message();
        let message = format!("{}\n\nEvent-Id: {}", event_line, event_id);

        let parent = self.get_latest_commit()?;
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        let meta = event.metadata();
        let author_name = if meta.author.name.is_empty() {
            "yx"
        } else {
            &meta.author.name
        };
        let author_email = if meta.author.email.is_empty() {
            "yx@localhost"
        } else {
            &meta.author.email
        };
        let time = git2::Time::new(meta.timestamp.as_epoch_secs(), 0);
        let sig = git2::Signature::new(author_name, author_email, &time)?;

        self.repo
            .commit(Some(&self.ref_name), &sig, &sig, &message, &tree, &parents)?;

        Ok(())
    }

    fn compact(&mut self, metadata: crate::domain::event_metadata::EventMetadata) -> Result<()> {
        let event = YakEvent::Compacted(metadata);
        self.append(&event)
    }

    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        let Some(latest) = self.get_latest_commit()? else {
            return Ok(Vec::new());
        };

        // Walk newest→oldest, collecting post-compaction events.
        // If we hit a Compacted commit, synthesize snapshot events
        // from its tree and stop walking.
        let mut post_compaction_events = Vec::new();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
        revwalk.push(latest.id())?;

        let mut compaction_tree: Option<git2::Tree> = None;

        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let full_message = commit.message().unwrap_or("");

            // Parse event from the first line of the commit message
            let first_line = full_message.lines().next().unwrap_or("").trim();
            if first_line.is_empty() {
                continue;
            }

            // Check for Compacted commit — stop walking and use its tree
            if first_line == "Compacted" {
                compaction_tree = Some(commit.tree()?);
                break;
            }

            match YakEvent::parse(first_line) {
                Ok(mut event) => {
                    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
                    let author = Author {
                        name: commit.author().name().unwrap_or("unknown").to_string(),
                        email: commit.author().email().unwrap_or("").to_string(),
                    };
                    let timestamp = Timestamp(commit.author().when().seconds());
                    let mut metadata = EventMetadata::new(author, timestamp);

                    // Extract Event-Id from commit message trailer,
                    // falling back to the commit SHA for legacy commits
                    metadata.event_id = Some(Self::extract_event_id(
                        full_message,
                        &commit.id().to_string(),
                    ));

                    // For FieldUpdated events, read the actual content
                    // from the git tree (not stored in commit message).
                    if let YakEvent::FieldUpdated(ref mut e, _) = event {
                        let tree = commit.tree().map_err(|err| {
                            anyhow::anyhow!(
                                "Failed to read tree for FieldUpdated event \
                                 (yak '{}', field '{}'): {}",
                                e.id,
                                e.field_name,
                                err
                            )
                        })?;
                        let yak_entry = tree.get_name(e.id.as_str()).ok_or_else(|| {
                            anyhow::anyhow!(
                                "Missing yak entry '{}' in tree for \
                                     FieldUpdated event (field '{}')",
                                e.id,
                                e.field_name
                            )
                        })?;
                        let yak_tree = self.repo.find_tree(yak_entry.id()).map_err(|err| {
                            anyhow::anyhow!(
                                "Failed to read yak subtree '{}' for \
                                     FieldUpdated event (field '{}'): {}",
                                e.id,
                                e.field_name,
                                err
                            )
                        })?;
                        let field_entry = yak_tree.get_name(&e.field_name).ok_or_else(|| {
                            anyhow::anyhow!(
                                "Missing field '{}' in yak '{}' subtree \
                                     for FieldUpdated event",
                                e.field_name,
                                e.id
                            )
                        })?;
                        let blob = self.repo.find_blob(field_entry.id()).map_err(|err| {
                            anyhow::anyhow!(
                                "Failed to read blob for field '{}' in \
                                     yak '{}': {}",
                                e.field_name,
                                e.id,
                                err
                            )
                        })?;
                        let content = std::str::from_utf8(blob.content()).map_err(|err| {
                            anyhow::anyhow!(
                                "Invalid UTF-8 in field '{}' of yak \
                                     '{}': {}",
                                e.field_name,
                                e.id,
                                err
                            )
                        })?;
                        e.content = content.to_string();
                    }

                    post_compaction_events.push(event.with_metadata(metadata));
                }
                Err(_) => continue, // Skip unparseable commits
            }
        }

        if let Some(tree) = compaction_tree {
            // Synthesize events from the compaction snapshot tree
            let mut snapshot_events = Vec::new();
            self.collect_compaction_events(&tree, &mut snapshot_events)?;

            // post_compaction_events are newest-first; reverse to
            // chronological then append after snapshot events
            post_compaction_events.reverse();
            snapshot_events.extend(post_compaction_events);
            Ok(snapshot_events)
        } else {
            // No compaction found — return all events chronologically
            post_compaction_events.reverse();
            Ok(post_compaction_events)
        }
    }

    fn reset_from_snapshot(&mut self, yaks: &[Yak]) -> Result<usize> {
        use super::migration::CURRENT_SCHEMA_VERSION;

        // Build root tree — all yaks flat at root
        let mut root_builder = self.repo.treebuilder(None)?;

        for yak in yaks {
            let yak_tree_oid = YakSubtreeBuilder::new(&self.repo)
                .name(yak.name.as_str())
                .state(&yak.state)
                .context(yak.context.as_deref().unwrap_or(""))
                .id(yak.id.as_str())
                .parent_id(yak.parent_id.as_ref().map(|p| p.as_str()))
                .metadata(&yak.created_by, yak.created_at)
                .custom_fields(&yak.fields)
                .build()?;
            root_builder.insert(yak.id.as_str(), yak_tree_oid, 0o040000)?;
        }

        // Add .schema-version
        let version_blob = self
            .repo
            .blob(CURRENT_SCHEMA_VERSION.to_string().as_bytes())?;
        root_builder.insert(".schema-version", version_blob, 0o100644)?;

        let tree_oid = root_builder.write()?;
        let tree = self.repo.find_tree(tree_oid)?;

        // Create commit
        let parent = self.get_latest_commit()?;
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        let sig = self
            .repo
            .signature()
            .or_else(|_| git2::Signature::now("yx", "yx@localhost"))?;

        self.repo.commit(
            Some(&self.ref_name),
            &sig,
            &sig,
            "Snapshot: rebuilt from disk",
            &tree,
            &parents,
        )?;

        Ok(yaks.len())
    }

    fn sync(
        &mut self,
        _bus: &mut crate::infrastructure::event_bus::EventBus,
        output: &dyn crate::domain::ports::DisplayPort,
    ) -> Result<()> {
        let repo_path = self
            .repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Cannot sync: bare repository"))?
            .to_path_buf();

        // 1. Fetch refs/notes/yaks from origin into a temporary peer ref
        let fetch_output = std::process::Command::new("git")
            .args(["fetch", "origin", "+refs/notes/yaks:refs/notes/yaks-peer"])
            .current_dir(&repo_path)
            .output();

        let has_origin = match fetch_output {
            Ok(out) => {
                if out.status.success() {
                    true
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    if stderr.contains("couldn't find remote ref") {
                        true // Remote has no ref yet (first sync)
                    } else {
                        false
                    }
                }
            }
            Err(_) => false,
        };

        if !has_origin {
            anyhow::bail!("Sync not configured");
        }

        // 2. Get local and peer events
        let local_events = EventStore::get_all_events(self)?;
        let peer = GitEventStore::with_ref_name(&repo_path, "refs/notes/yaks-peer")?;
        let peer_events = EventStore::get_all_events(&peer)?;

        let merge = super::merge_event_streams(&local_events, &peer_events);

        if merge.pulled > 0 {
            // Delete the local ref and replay all events in sorted order
            if let Ok(mut r) = self.repo.find_reference(&self.ref_name) {
                r.delete()?;
            }

            for event in &merge.events {
                self.append(event)?;
            }
        }

        output.info(&format!(
            "Pulled {} events, pushed {} events",
            merge.pulled, merge.pushed
        ));

        // 3. Push refs/notes/yaks back to origin
        if self.repo.refname_to_id(&self.ref_name).is_ok() {
            let push_output = std::process::Command::new("git")
                .args(["push", "origin", "+refs/notes/yaks:refs/notes/yaks"])
                .current_dir(&repo_path)
                .output()?;

            if !push_output.status.success() {
                let stderr = String::from_utf8_lossy(&push_output.stderr);
                anyhow::bail!("Failed to push to origin: {}", stderr.trim());
            }
        }

        // 4. Clean up the temporary peer ref
        let _ = self
            .repo
            .find_reference("refs/notes/yaks-peer")
            .and_then(|mut r| r.delete());

        Ok(())
    }
}

impl EventStoreReader for GitEventStore {
    fn get_all_events(&self) -> Result<Vec<YakEvent>> {
        EventStore::get_all_events(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
    use crate::domain::events::FieldUpdatedEvent;
    use crate::domain::slug::{Name, YakId};
    use crate::domain::AddedEvent;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitEventStore) {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        // Configure git user for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@test.com").unwrap();

        let store = GitEventStore::from_repo(repo);
        (tmp, store)
    }

    #[test]
    fn append_creates_commit_on_refs_notes_yaks() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Verify ref exists
        let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = store.repo.find_commit(oid).unwrap();
        let message = commit.message().unwrap();
        // First line is the event description
        assert!(
            message.starts_with("Added: \"test\" \"test-a1b2\""),
            "Commit message should start with event description, got: {}",
            message
        );
        // Should contain an Event-Id trailer
        assert!(
            message.contains("Event-Id: "),
            "Commit message should contain Event-Id trailer, got: {}",
            message
        );
    }

    #[test]
    fn added_with_id_keys_tree_entry_by_id() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Tree entry should be keyed by id, not name
        assert!(
            tree.get_name("test-a1b2").is_some(),
            "Expected tree entry keyed by id 'test-a1b2'"
        );
        assert!(
            tree.get_name("test").is_none(),
            "Should not have tree entry keyed by name 'test'"
        );
    }

    #[test]
    fn state_update_after_add_uses_same_tree_entry() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: "state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Should have exactly one entry, keyed by id
        assert_eq!(
            tree.len(),
            1,
            "Expected exactly 1 tree entry, got {}",
            tree.len()
        );

        let entry = tree.get_name("test-a1b2").unwrap();
        let subtree = entry.to_object(&store.repo).unwrap();
        let subtree = subtree.as_tree().unwrap();

        // Verify state was updated
        let state_entry = subtree.get_name("state").unwrap();
        let state_blob = state_entry.to_object(&store.repo).unwrap();
        let state_content = std::str::from_utf8(state_blob.as_blob().unwrap().content()).unwrap();
        assert_eq!(state_content, "wip");
    }

    #[test]
    fn added_with_parent_id_stores_flat_with_parent_id_blob() {
        let (_tmp, mut store) = setup_test_repo();

        // Add parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Add child under parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Root should have two entries: parent and child (flat)
        assert_eq!(tree.len(), 2);

        // Both at root level
        assert!(
            tree.get_name("parent-a1b2").is_some(),
            "Expected parent at root"
        );
        assert!(
            tree.get_name("child-c3d4").is_some(),
            "Expected child at root"
        );

        // Child should have parent_id blob
        let child_entry = tree.get_name("child-c3d4").unwrap();
        let child_tree = store.repo.find_tree(child_entry.id()).unwrap();
        let parent_id_blob = child_tree.get_name("parent_id").unwrap();
        let parent_id = store.repo.find_blob(parent_id_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(parent_id.content()).unwrap(),
            "parent-a1b2"
        );

        // Parent should NOT have parent_id blob
        let parent_entry = tree.get_name("parent-a1b2").unwrap();
        let parent_tree = store.repo.find_tree(parent_entry.id()).unwrap();
        assert!(
            parent_tree.get_name("parent_id").is_none(),
            "Root yak should not have parent_id blob"
        );
    }

    #[test]
    fn snapshot_events_synthesizes_added_for_each_yak() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        // Should have an Added event with regenerated ID
        let added = events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(_, _)))
            .unwrap();
        if let YakEvent::Added(e, _) = added {
            assert_eq!(e.name, Name::from("test"));
            assert!(
                e.id.as_str().starts_with("test-"),
                "Expected regenerated ID starting with 'test-', got '{}'",
                e.id
            );
            assert!(e.parent_id.is_none());
        }
    }

    #[test]
    fn snapshot_events_includes_state_and_context() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: "state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: "context.md".to_string(),
                    content: "some notes".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        assert!(
            events
                .iter()
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == "state" && f.content == "wip")),
            "Expected FieldUpdated event for state 'wip'"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == "context.md" && f.content == "some notes")),
            "Expected FieldUpdated event for context.md"
        );
    }

    #[test]
    fn snapshot_events_skips_state_when_todo() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == "state")),
            "Should not emit FieldUpdated for state when state is 'todo'"
        );
    }

    #[test]
    fn snapshot_events_regenerates_ids_for_legacy_yaks() {
        let (_tmp, mut store) = setup_test_repo();

        // Simulate a legacy yak where the tree key is a plain slug
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("dx"),
                    id: YakId::from("dx"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();
        let added = events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(_, _)))
            .unwrap();

        if let YakEvent::Added(e, _) = added {
            // Should get a proper ID with suffix, not plain "dx"
            assert!(
                e.id.as_str().starts_with("dx-") && e.id.as_str().len() > 3,
                "Expected regenerated ID like 'dx-xxxx', got '{}'",
                e.id
            );
        }
    }

    #[test]
    fn snapshot_events_handles_flat_yaks_with_parent_id() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();
        let added_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, YakEvent::Added(_, _)))
            .collect();

        assert_eq!(added_events.len(), 2, "Expected 2 Added events");

        // Find parent and child by name
        let parent_event = added_events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(a, _) if a.name == "parent"))
            .expect("Expected parent Added event");
        let child_event = added_events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(a, _) if a.name == "child"))
            .expect("Expected child Added event");

        if let (YakEvent::Added(parent, _), YakEvent::Added(child, _)) = (parent_event, child_event)
        {
            assert!(parent.parent_id.is_none());
            // Child reads parent_id from blob in flat tree
            assert_eq!(child.parent_id.as_ref(), Some(&parent.id));
        }
    }

    #[test]
    fn snapshot_events_includes_custom_fields() {
        let (_tmp, mut store) = setup_test_repo();

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                crate::domain::events::FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: "plan".to_string(),
                    content: "step 1".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        assert!(
            events.iter().any(
                |e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == "plan" && f.content == "step 1")
            ),
            "Expected FieldUpdated event for 'plan'"
        );
    }

    #[test]
    fn snapshot_events_empty_tree() {
        let (_tmp, store) = setup_test_repo();
        let events = store.snapshot_events().unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn snapshot_events_reads_metadata_from_tree() {
        use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};

        let (_tmp, mut store) = setup_test_repo();

        let metadata = EventMetadata::new(
            Author {
                name: "Snapshot Author".to_string(),
                email: "snap@test.com".to_string(),
            },
            Timestamp(1708300800),
        );

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                metadata.clone(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();
        let added = events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(..)))
            .unwrap();
        assert_eq!(added.metadata().author.name, "Snapshot Author");
        assert_eq!(added.metadata().timestamp, Timestamp(1708300800));
    }

    #[test]
    fn reset_from_snapshot_builds_correct_tree() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let mut fields = HashMap::new();
        fields.insert("plan".to_string(), "step 1".to_string());

        let yak1 = Yak {
            id: YakId::from("yak1-a1b2"),
            name: Name::from("First Yak"),
            parent_id: None,
            state: "todo".to_string(),
            context: Some("some context".to_string()),
            fields: fields.clone(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let yak2 = Yak {
            id: YakId::from("yak2-c3d4"),
            name: Name::from("Second Yak"),
            parent_id: None,
            state: "wip".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let count = store.reset_from_snapshot(&[yak1, yak2]).unwrap();
        assert_eq!(count, 2);

        // Verify the git tree structure
        let tree = store.get_current_tree().unwrap().unwrap();

        // Check yak1 subtree
        let yak1_entry = tree.get_name("yak1-a1b2").unwrap();
        let yak1_tree = store.repo.find_tree(yak1_entry.id()).unwrap();

        let state_blob = yak1_tree.get_name("state").unwrap();
        let state = store.repo.find_blob(state_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(state.content()).unwrap(), "todo");

        let context_blob = yak1_tree.get_name("context.md").unwrap();
        let context = store.repo.find_blob(context_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(context.content()).unwrap(),
            "some context"
        );

        let name_blob = yak1_tree.get_name("name").unwrap();
        let name = store.repo.find_blob(name_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(name.content()).unwrap(), "First Yak");

        let id_blob = yak1_tree.get_name("id").unwrap();
        let id = store.repo.find_blob(id_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(id.content()).unwrap(), "yak1-a1b2");

        let plan_blob = yak1_tree.get_name("plan").unwrap();
        let plan = store.repo.find_blob(plan_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(plan.content()).unwrap(), "step 1");

        // Check yak2 subtree
        let yak2_entry = tree.get_name("yak2-c3d4").unwrap();
        let yak2_tree = store.repo.find_tree(yak2_entry.id()).unwrap();

        let state2_blob = yak2_tree.get_name("state").unwrap();
        let state2 = store.repo.find_blob(state2_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(state2.content()).unwrap(), "wip");

        let context2_blob = yak2_tree.get_name("context.md").unwrap();
        let context2 = store.repo.find_blob(context2_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(context2.content()).unwrap(), "");

        // Check .schema-version
        let schema_blob = tree.get_name(".schema-version").unwrap();
        let schema = store.repo.find_blob(schema_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(schema.content()).unwrap(), "4");
    }

    #[test]
    fn reset_from_snapshot_handles_children() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let child = Yak {
            id: YakId::from("child-x1y2"),
            name: Name::from("Child Yak"),
            parent_id: Some(YakId::from("parent-a1b2")),
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let parent = Yak {
            id: YakId::from("parent-a1b2"),
            name: Name::from("Parent Yak"),
            parent_id: None,
            state: "wip".to_string(),
            context: Some("parent context".to_string()),
            fields: HashMap::new(),
            children: vec![YakId::from("child-x1y2")],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        store.reset_from_snapshot(&[parent, child]).unwrap();

        // Verify tree structure
        let tree = store.get_current_tree().unwrap().unwrap();

        // Root should have parent + child + .schema-version (flat)
        assert_eq!(tree.len(), 3);

        // Both at root level
        assert!(tree.get_name("parent-a1b2").is_some());
        assert!(tree.get_name("child-x1y2").is_some());

        // Parent should have its own blobs, no child subtree
        let parent_entry = tree.get_name("parent-a1b2").unwrap();
        let parent_tree = store.repo.find_tree(parent_entry.id()).unwrap();
        assert!(parent_tree.get_name("state").is_some());
        assert!(parent_tree.get_name("context.md").is_some());
        assert!(parent_tree.get_name("name").is_some());
        assert!(parent_tree.get_name("id").is_some());
        assert!(
            parent_tree.get_name("parent_id").is_none(),
            "Root yak should not have parent_id"
        );

        // Child at root with parent_id blob
        let child_entry = tree.get_name("child-x1y2").unwrap();
        let child_tree = store.repo.find_tree(child_entry.id()).unwrap();

        let child_name_blob = child_tree.get_name("name").unwrap();
        let child_name = store.repo.find_blob(child_name_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(child_name.content()).unwrap(),
            "Child Yak"
        );

        let parent_id_blob = child_tree.get_name("parent_id").unwrap();
        let parent_id = store.repo.find_blob(parent_id_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(parent_id.content()).unwrap(),
            "parent-a1b2"
        );
    }

    #[test]
    fn reset_from_snapshot_parents_to_existing_commit() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        // First, add a yak via append (creates a commit)
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("initial"),
                    id: YakId::from("initial-z9z9"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let first_commit_oid = store.get_latest_commit().unwrap().unwrap().id();

        // Now call reset_from_snapshot
        let yak = Yak {
            id: YakId::from("snapshot-a1b2"),
            name: Name::from("Snapshot Yak"),
            parent_id: None,
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        store.reset_from_snapshot(&[yak]).unwrap();

        // Verify the new commit has a parent
        let latest = store.get_latest_commit().unwrap().unwrap();
        assert_eq!(latest.parent_count(), 1);
        assert_eq!(latest.parent_id(0).unwrap(), first_commit_oid);
    }

    #[test]
    fn reset_from_snapshot_returns_yak_count() {
        use std::collections::HashMap;

        let (_tmp, mut store) = setup_test_repo();

        let yak1 = Yak {
            id: YakId::from("yak1-a1b2"),
            name: Name::from("Yak One"),
            parent_id: None,
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let yak2 = Yak {
            id: YakId::from("yak2-c3d4"),
            name: Name::from("Yak Two"),
            parent_id: None,
            state: "wip".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let yak3 = Yak {
            id: YakId::from("yak3-e5f6"),
            name: Name::from("Yak Three"),
            parent_id: None,
            state: "done".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };

        let count = store.reset_from_snapshot(&[yak1, yak2, yak3]).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn reset_from_snapshot_empty_list() {
        let (_tmp, mut store) = setup_test_repo();

        let count = store.reset_from_snapshot(&[]).unwrap();
        assert_eq!(count, 0);

        // Verify a commit was created
        let tree = store.get_current_tree().unwrap().unwrap();

        // Should only have .schema-version
        assert_eq!(tree.len(), 1);
        let schema_blob = tree.get_name(".schema-version").unwrap();
        let schema = store.repo.find_blob(schema_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(schema.content()).unwrap(), "4");
    }

    #[test]
    fn rename_nested_yak_updates_correct_entry() {
        let (_tmp, mut store) = setup_test_repo();

        // Add parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Add child under parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Rename the child
        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("child-c3d4"),
                    field_name: "name".to_string(),
                    content: "renamed child".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Root should have two entries: parent + child (flat)
        let root_entries: Vec<_> = tree
            .iter()
            .filter(|e| e.kind() == Some(git2::ObjectType::Tree))
            .collect();
        assert_eq!(
            root_entries.len(),
            2,
            "Expected 2 root tree entries, got {}",
            root_entries.len()
        );

        // Verify the child's name was updated at root level
        let child_entry = tree.get_name("child-c3d4").unwrap();
        let child_tree = store.repo.find_tree(child_entry.id()).unwrap();

        let name_blob = child_tree.get_name("name").unwrap();
        let name = store.repo.find_blob(name_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(name.content()).unwrap(),
            "renamed child"
        );
    }

    #[test]
    fn state_update_nested_yak_updates_correct_entry() {
        let (_tmp, mut store) = setup_test_repo();

        // Add parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("parent"),
                    id: YakId::from("parent-a1b2"),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Add child under parent
        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("child"),
                    id: YakId::from("child-c3d4"),
                    parent_id: Some(YakId::from("parent-a1b2")),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        // Update state of child (now at root level)
        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("child-c3d4"),
                    field_name: "state".to_string(),
                    content: "done".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Root should have two entries: parent + child (flat)
        let root_entries: Vec<_> = tree
            .iter()
            .filter(|e| e.kind() == Some(git2::ObjectType::Tree))
            .collect();
        assert_eq!(root_entries.len(), 2);

        // Verify state was updated at root level
        let child_entry = tree.get_name("child-c3d4").unwrap();
        let child_tree = store.repo.find_tree(child_entry.id()).unwrap();

        let state_blob = child_tree.get_name("state").unwrap();
        let state = store.repo.find_blob(state_blob.id()).unwrap();
        assert_eq!(std::str::from_utf8(state.content()).unwrap(), "done");
    }

    #[test]
    fn append_uses_event_metadata_for_commit_signature() {
        use crate::domain::event_metadata::{Author, Timestamp};

        let (_tmp, mut store) = setup_test_repo();

        let metadata = EventMetadata::new(
            Author {
                name: "Custom Author".to_string(),
                email: "custom@example.com".to_string(),
            },
            Timestamp(1708300800),
        );

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                metadata,
            ))
            .unwrap();

        let oid = store.repo.refname_to_id("refs/notes/yaks").unwrap();
        let commit = store.repo.find_commit(oid).unwrap();
        assert_eq!(commit.author().name().unwrap(), "Custom Author");
        assert_eq!(commit.author().email().unwrap(), "custom@example.com");
        assert_eq!(commit.author().when().seconds(), 1708300800);
    }

    #[test]
    fn get_all_events_populates_metadata_from_commits() {
        use crate::domain::event_metadata::{Author, Timestamp};

        let (_tmp, mut store) = setup_test_repo();

        let metadata = EventMetadata::new(
            Author {
                name: "Reader Test".to_string(),
                email: "reader@test.com".to_string(),
            },
            Timestamp(1708300800),
        );

        store
            .append(&YakEvent::Added(
                AddedEvent {
                    name: Name::from("test"),
                    id: YakId::from("test-a1b2"),
                    parent_id: None,
                },
                metadata.clone(),
            ))
            .unwrap();

        let events = EventStore::get_all_events(&store).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].metadata().author.name, "Reader Test");
        assert_eq!(events[0].metadata().author.email, "reader@test.com");
        assert_eq!(events[0].metadata().timestamp, Timestamp(1708300800));
    }

    mod sync {
        use super::*;
        use crate::adapters::make_test_display;
        use crate::infrastructure::event_bus::EventBus;

        fn make_event(name: &str, id: &str) -> YakEvent {
            YakEvent::Added(
                AddedEvent {
                    name: Name::from(name),
                    id: YakId::from(id),
                    parent_id: None,
                },
                EventMetadata::default_legacy(),
            )
        }

        fn all_events(store: &GitEventStore) -> Vec<YakEvent> {
            EventStore::get_all_events(store).unwrap()
        }

        /// Set up a bare "origin" repo and a "local" repo with origin as remote
        fn setup_origin_and_local() -> (TempDir, TempDir, GitEventStore) {
            // Create bare origin
            let origin_dir = TempDir::new().unwrap();
            Repository::init_bare(origin_dir.path()).unwrap();

            // Create local repo
            let local_dir = TempDir::new().unwrap();
            let local_repo = Repository::init(local_dir.path()).unwrap();

            // Configure git user
            let mut config = local_repo.config().unwrap();
            config.set_str("user.name", "test").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();

            // Add origin remote
            local_repo
                .remote("origin", origin_dir.path().to_str().unwrap())
                .unwrap();

            let store = GitEventStore::from_repo(local_repo);
            (origin_dir, local_dir, store)
        }

        #[test]
        fn sync_pulls_events_from_origin() {
            let (origin_dir, _local_dir, mut local_store) = setup_origin_and_local();

            // Add events directly to origin's refs/notes/yaks
            let mut origin_store = GitEventStore::new(origin_dir.path()).unwrap();
            origin_store
                .append(&make_event("from-origin", "from-origin-a1b2"))
                .unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local_store.sync(&mut bus, &output).unwrap();

            let events = all_events(&local_store);
            assert_eq!(
                events.len(),
                1,
                "local should have pulled 1 event from origin"
            );
        }

        #[test]
        fn sync_pushes_events_to_origin() {
            let (origin_dir, _local_dir, mut local_store) = setup_origin_and_local();

            // Add event to local
            local_store
                .append(&make_event("from-local", "from-local-a1b2"))
                .unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local_store.sync(&mut bus, &output).unwrap();

            // Check origin has the event
            let origin_store = GitEventStore::new(origin_dir.path()).unwrap();
            let events = all_events(&origin_store);
            assert_eq!(
                events.len(),
                1,
                "origin should have 1 event pushed from local"
            );
        }

        #[test]
        fn sync_exchanges_events_bidirectionally() {
            let (origin_dir, _local_dir, mut local_store) = setup_origin_and_local();

            // Add event to origin
            let mut origin_store = GitEventStore::new(origin_dir.path()).unwrap();
            origin_store
                .append(&make_event("from-origin", "from-origin-a1b2"))
                .unwrap();

            // Add event to local
            local_store
                .append(&make_event("from-local", "from-local-c3d4"))
                .unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local_store.sync(&mut bus, &output).unwrap();

            // Local should have both events
            let local_events = all_events(&local_store);
            assert_eq!(local_events.len(), 2, "local should have 2 events");

            // Origin should have both events (pushed back)
            let origin_store = GitEventStore::new(origin_dir.path()).unwrap();
            let origin_events = all_events(&origin_store);
            assert_eq!(origin_events.len(), 2, "origin should have 2 events");
        }

        #[test]
        fn sync_rebases_divergent_histories_into_linear_chain() {
            let (origin_dir, _local_dir, mut local_store) = setup_origin_and_local();
            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            // Push shared yak to origin
            local_store
                .append(&make_event("shared", "shared-a1b2"))
                .unwrap();
            local_store.sync(&mut bus, &output).unwrap();

            // Add event directly to origin (simulates another user)
            let mut origin_store = GitEventStore::new(origin_dir.path()).unwrap();
            origin_store
                .append(&make_event("from-origin", "from-origin-c3d4"))
                .unwrap();

            // Add event to local (now diverged from origin)
            local_store
                .append(&make_event("from-local", "from-local-e5f6"))
                .unwrap();

            // Sync should rebase into linear history
            local_store.sync(&mut bus, &output).unwrap();

            // All three yaks in final tree
            let tree = local_store.get_current_tree().unwrap().unwrap();
            assert!(tree.get_name("shared-a1b2").is_some());
            assert!(tree.get_name("from-origin-c3d4").is_some());
            assert!(tree.get_name("from-local-e5f6").is_some());

            // Every commit has at most 1 parent (linear, no merge commits)
            let events = all_events(&local_store);
            assert_eq!(events.len(), 3);

            let tip = local_store.get_latest_commit().unwrap().unwrap();
            assert_eq!(
                tip.parent_count(),
                1,
                "tip should have 1 parent (linear history, not merge)"
            );
        }

        #[test]
        fn sync_fast_forwards_when_local_is_behind() {
            let (origin_dir, _local_dir, mut local_store) = setup_origin_and_local();
            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local_store
                .append(&make_event("shared", "shared-a1b2"))
                .unwrap();
            local_store.sync(&mut bus, &output).unwrap();

            // Add event to origin (local is now behind)
            let mut origin_store = GitEventStore::new(origin_dir.path()).unwrap();
            origin_store.append(&make_event("new", "new-c3d4")).unwrap();

            local_store.sync(&mut bus, &output).unwrap();

            let events = all_events(&local_store);
            assert_eq!(events.len(), 2);

            // Linear history (1 parent, no merge)
            let tip = local_store.get_latest_commit().unwrap().unwrap();
            assert_eq!(tip.parent_count(), 1);
        }

        #[test]
        fn sync_cleans_up_peer_ref() {
            let (_origin_dir, _local_dir, mut local_store) = setup_origin_and_local();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local_store.sync(&mut bus, &output).unwrap();

            // The temporary peer ref should be cleaned up
            assert!(
                local_store
                    .repo
                    .find_reference("refs/notes/yaks-peer")
                    .is_err(),
                "refs/notes/yaks-peer should be cleaned up after sync"
            );
        }
    }

    #[test]
    fn get_all_events_errors_when_field_content_unreadable() {
        let (_tmp, store) = setup_test_repo();

        // Manually create a commit with a FieldUpdated message
        // but an empty tree (no yak subtree), simulating corruption.
        let empty_tree_oid = store.repo.treebuilder(None).unwrap().write().unwrap();
        let empty_tree = store.repo.find_tree(empty_tree_oid).unwrap();

        let sig = git2::Signature::now("test", "test@test.com").unwrap();
        store
            .repo
            .commit(
                Some("refs/notes/yaks"),
                &sig,
                &sig,
                "FieldUpdated: \"missing-yak-a1b2\" \"state\"\n\nEvent-Id: test-event-1",
                &empty_tree,
                &[],
            )
            .unwrap();

        // get_all_events should return an error, not silently
        // return FieldUpdated with empty content.
        let result = EventStore::get_all_events(&store);
        assert!(
            result.is_err(),
            "Expected error when FieldUpdated tree blob is unreadable, \
             but got Ok with {} events",
            result.unwrap().len()
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("missing-yak-a1b2"),
            "Error should mention the yak id, got: {}",
            err_msg
        );
        assert!(
            err_msg.contains("state"),
            "Error should mention the field name, got: {}",
            err_msg
        );
    }
}
