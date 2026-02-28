use anyhow::Result;
use git2::Repository;
use std::path::Path;

use crate::domain::ports::{EventStore, EventStoreReader};
use crate::domain::YakEvent;

use super::commit;
use super::tree;

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

    /// Access the underlying repository.
    pub(super) fn repo(&self) -> &Repository {
        &self.repo
    }

    /// Access the ref name.
    pub(super) fn ref_name(&self) -> &str {
        &self.ref_name
    }

    /// Get the latest commit on refs/notes/yaks, if any
    pub(super) fn get_latest_commit(&self) -> Result<Option<git2::Commit<'_>>> {
        match self.repo.refname_to_id(&self.ref_name) {
            Ok(oid) => Ok(Some(self.repo.find_commit(oid)?)),
            Err(_) => Ok(None),
        }
    }

    /// Get the current tree from refs/notes/yaks, if any
    pub(super) fn get_current_tree(&self) -> Result<Option<git2::Tree<'_>>> {
        match self.get_latest_commit()? {
            Some(commit) => Ok(Some(commit.tree()?)),
            None => Ok(None),
        }
    }

    /// Read the current git tree state and synthesize domain events.
    /// All yak IDs are regenerated using `generate_id(name, parent_id)`,
    /// making this suitable for repairing inconsistent data.
    pub fn snapshot_events(&self) -> Result<Vec<YakEvent>> {
        let tree = self.get_current_tree()?;
        let Some(tree) = tree else {
            return Ok(Vec::new());
        };

        let snapshots = tree::read_snapshots_from_tree(&self.repo, &tree)?;
        let mut events = Vec::new();

        for snap in &snapshots {
            // Added event with metadata from the snapshot
            let metadata = crate::domain::event_metadata::EventMetadata::new(
                snap.created_by.clone(),
                snap.created_at,
            );
            events.push(YakEvent::Added(
                crate::domain::events::AddedEvent {
                    name: snap.name.clone(),
                    id: snap.id.clone(),
                    parent_id: snap.parent_id.clone(),
                },
                metadata,
            ));

            // State (skip default "todo")
            if snap.state != "todo" {
                events.push(YakEvent::FieldUpdated(
                    crate::domain::events::FieldUpdatedEvent {
                        id: snap.id.clone(),
                        field_name: ".state".to_string(),
                        content: snap.state.clone(),
                    },
                    crate::domain::event_metadata::EventMetadata::default_legacy(),
                ));
            }

            // Context (skip empty)
            if let Some(ref ctx) = snap.context {
                if !ctx.is_empty() {
                    events.push(YakEvent::FieldUpdated(
                        crate::domain::events::FieldUpdatedEvent {
                            id: snap.id.clone(),
                            field_name: ".context.md".to_string(),
                            content: ctx.clone(),
                        },
                        crate::domain::event_metadata::EventMetadata::default_legacy(),
                    ));
                }
            }

            // Custom fields
            for (field_name, content) in &snap.fields {
                events.push(YakEvent::FieldUpdated(
                    crate::domain::events::FieldUpdatedEvent {
                        id: snap.id.clone(),
                        field_name: field_name.clone(),
                        content: content.clone(),
                    },
                    crate::domain::event_metadata::EventMetadata::default_legacy(),
                ));
            }
        }

        Ok(events)
    }
}

impl EventStore for GitEventStore {
    fn append(&mut self, event: &YakEvent) -> Result<()> {
        let event = super::ensure_event_id(event.clone());
        let event_id = event.metadata().event_id.clone().unwrap();

        // Idempotent: skip if we already have a commit with this event_id
        if commit::has_event_id(&self.repo, &self.ref_name, &event_id)? {
            return Ok(());
        }

        let current_tree = self.get_current_tree()?;

        let tree_oid = tree::build_tree_from_event(&self.repo, &event, current_tree.as_ref())?;

        // Ensure .schema-version is stamped on every commit's tree.
        // This is critical for sync: peer refs must be identifiable by version.
        let tree = {
            let built_tree = self.repo.find_tree(tree_oid)?;
            if built_tree.get_name(".schema-version").is_some() {
                built_tree
            } else {
                use super::migration::CURRENT_SCHEMA_VERSION;
                let version_blob = self
                    .repo
                    .blob(CURRENT_SCHEMA_VERSION.to_string().as_bytes())?;
                let mut builder = self.repo.treebuilder(Some(&built_tree))?;
                builder.insert(".schema-version", version_blob, 0o100644)?;
                let stamped_oid = builder.write()?;
                self.repo.find_tree(stamped_oid)?
            }
        };

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

    fn wipe(&mut self) -> Result<()> {
        let delete_result = self.repo.find_reference(&self.ref_name);
        if let Ok(mut reference) = delete_result {
            reference.delete()?;
        }
        Ok(())
    }

    fn compact(&mut self, metadata: crate::domain::event_metadata::EventMetadata) -> Result<()> {
        if self.get_latest_commit()?.is_none() {
            anyhow::bail!("Cannot compact an empty event store");
        }
        let snapshots = {
            let tree = self.get_current_tree()?.unwrap();
            tree::read_snapshots_from_tree(&self.repo, &tree)?
        };
        let event = YakEvent::Compacted(snapshots, metadata);
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
        let mut compaction_metadata: Option<crate::domain::event_metadata::EventMetadata> = None;

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
                use crate::domain::event_metadata::{Author, EventMetadata, Timestamp};
                let author = Author {
                    name: commit.author().name().unwrap_or("unknown").to_string(),
                    email: commit.author().email().unwrap_or("").to_string(),
                };
                let timestamp = Timestamp(commit.author().when().seconds());
                let mut metadata = EventMetadata::new(author, timestamp);
                metadata.event_id = Some(commit::extract_event_id(
                    full_message,
                    &commit.id().to_string(),
                ));
                compaction_metadata = Some(metadata);
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
                    metadata.event_id = Some(commit::extract_event_id(
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
            let metadata = compaction_metadata.unwrap();

            // Read snapshots from the compaction tree
            let snapshots = tree::read_snapshots_from_tree(&self.repo, &tree)?;

            let mut result = Vec::new();
            result.push(YakEvent::Compacted(snapshots, metadata));

            // post_compaction_events are newest-first; reverse to
            // chronological then append after the Compacted event
            post_compaction_events.reverse();
            result.extend(post_compaction_events);
            Ok(result)
        } else {
            // No compaction found — return all events chronologically
            post_compaction_events.reverse();
            Ok(post_compaction_events)
        }
    }

    fn sync(
        &mut self,
        bus: &mut crate::infrastructure::event_bus::EventBus,
        output: &dyn crate::domain::ports::DisplayPort,
    ) -> Result<()> {
        super::sync::sync_with_remote(self, bus, output)
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
    use crate::domain::event_metadata::EventMetadata;
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
    fn append_stamps_schema_version_on_first_event() {
        use crate::adapters::event_store::migration::{
            read_schema_version, EventStoreLocation, CURRENT_SCHEMA_VERSION,
        };

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

        let location = EventStoreLocation {
            repo: &store.repo,
            ref_name: "refs/notes/yaks",
        };
        let version = read_schema_version(&location).unwrap();
        assert_eq!(
            version,
            Some(CURRENT_SCHEMA_VERSION),
            "First append should stamp the current schema version"
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
                    field_name: ".state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();

        // Should have exactly two entries: the yak and .schema-version
        assert_eq!(
            tree.len(),
            2,
            "Expected exactly 2 tree entries (yak + .schema-version), got {}",
            tree.len()
        );

        let entry = tree.get_name("test-a1b2").unwrap();
        let subtree = entry.to_object(&store.repo).unwrap();
        let subtree = subtree.as_tree().unwrap();

        // Verify state was updated
        let state_entry = subtree.get_name(".state").unwrap();
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

        // Root should have three entries: parent, child, and .schema-version
        assert_eq!(tree.len(), 3);

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
        let parent_id_blob = child_tree.get_name(".parent_id").unwrap();
        let parent_id = store.repo.find_blob(parent_id_blob.id()).unwrap();
        assert_eq!(
            std::str::from_utf8(parent_id.content()).unwrap(),
            "parent-a1b2"
        );

        // Parent should NOT have parent_id blob
        let parent_entry = tree.get_name("parent-a1b2").unwrap();
        let parent_tree = store.repo.find_tree(parent_entry.id()).unwrap();
        assert!(
            parent_tree.get_name(".parent_id").is_none(),
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

        // Should have an Added event with preserved ID
        let added = events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(_, _)))
            .unwrap();
        if let YakEvent::Added(e, _) = added {
            assert_eq!(e.name, Name::from("test"));
            assert_eq!(e.id, YakId::from("test-a1b2"));
            assert!(e.parent_id.is_none());
        }
    }

    #[test]
    fn snapshot_events_preserves_existing_yak_ids() {
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
        let added = events
            .iter()
            .find(|e| matches!(e, YakEvent::Added(_, _)))
            .unwrap();
        if let YakEvent::Added(e, _) = added {
            assert_eq!(
                e.id,
                YakId::from("test-a1b2"),
                "snapshot_events should preserve existing yak ID, not regenerate"
            );
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
                    field_name: ".state".to_string(),
                    content: "wip".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        store
            .append(&YakEvent::FieldUpdated(
                FieldUpdatedEvent {
                    id: YakId::from("test-a1b2"),
                    field_name: ".context.md".to_string(),
                    content: "some notes".to_string(),
                },
                EventMetadata::default_legacy(),
            ))
            .unwrap();

        let events = store.snapshot_events().unwrap();

        assert!(
            events
                .iter()
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == ".state" && f.content == "wip")),
            "Expected FieldUpdated event for state 'wip'"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == ".context.md" && f.content == "some notes")),
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
                .any(|e| matches!(e, YakEvent::FieldUpdated(f, _) if f.field_name == ".state")),
            "Should not emit FieldUpdated for state when state is 'todo'"
        );
    }

    #[test]
    fn snapshot_events_preserves_legacy_yak_ids() {
        let (_tmp, mut store) = setup_test_repo();

        // Legacy yak with plain slug ID (no suffix).
        // Migrations (v2→v3) should have added proper IDs, but if
        // a legacy yak still has a plain ID, snapshot_events
        // preserves it as-is rather than regenerating.
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
            assert_eq!(
                e.id,
                YakId::from("dx"),
                "snapshot_events should preserve existing ID, even legacy plain slugs"
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
                    field_name: ".name".to_string(),
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

        let name_blob = child_tree.get_name(".name").unwrap();
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
                    field_name: ".state".to_string(),
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

        let state_blob = child_tree.get_name(".state").unwrap();
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
    fn compact_preserves_schema_version_in_tree() {
        use crate::adapters::event_store::migration::CURRENT_SCHEMA_VERSION;

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

        store.compact(EventMetadata::default_legacy()).unwrap();

        let tree = store.get_current_tree().unwrap().unwrap();
        let schema_entry = tree
            .get_name(".schema-version")
            .expect(".schema-version should exist in tree after compact");
        let blob = store.repo.find_blob(schema_entry.id()).unwrap();
        let content = std::str::from_utf8(blob.content()).unwrap();
        assert_eq!(
            content,
            CURRENT_SCHEMA_VERSION.to_string(),
            "Schema version should be {} after compact",
            CURRENT_SCHEMA_VERSION
        );
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

    mod peer_schema_version {
        use super::*;
        use crate::adapters::event_store::migration::{
            write_schema_version, EventStoreLocation, CURRENT_SCHEMA_VERSION,
        };
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

        fn setup_origin_and_local() -> (TempDir, TempDir, GitEventStore) {
            let origin_dir = TempDir::new().unwrap();
            Repository::init_bare(origin_dir.path()).unwrap();

            let local_dir = TempDir::new().unwrap();
            let local_repo = Repository::init(local_dir.path()).unwrap();

            let mut config = local_repo.config().unwrap();
            config.set_str("user.name", "test").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();

            local_repo
                .remote("origin", origin_dir.path().to_str().unwrap())
                .unwrap();

            let store = GitEventStore::from_repo(local_repo);
            (origin_dir, local_dir, store)
        }

        /// Stamp a specific schema version on the origin's refs/notes/yaks ref.
        fn stamp_origin_schema_version(origin_dir: &TempDir, version: u32) {
            let repo = Repository::open(origin_dir.path()).unwrap();
            let location = EventStoreLocation {
                repo: &repo,
                ref_name: "refs/notes/yaks",
            };
            write_schema_version(&location, version).unwrap();
        }

        #[test]
        fn sync_refuses_when_peer_schema_version_is_newer() {
            let (origin_dir, _local_dir, mut local_store) = setup_origin_and_local();

            // Add an event so origin has a ref
            let mut origin_store = GitEventStore::new(origin_dir.path()).unwrap();
            origin_store
                .append(&make_event("from-origin", "from-origin-a1b2"))
                .unwrap();

            // Stamp a future schema version on origin
            stamp_origin_schema_version(&origin_dir, CURRENT_SCHEMA_VERSION + 1);

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            let result = local_store.sync(&mut bus, &output);
            assert!(result.is_err(), "Sync should fail when peer is newer");
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("Please update yx"),
                "Error should tell user to update, got: {}",
                err
            );
        }

        #[test]
        fn sync_succeeds_when_peer_schema_version_matches() {
            let (origin_dir, _local_dir, mut local_store) = setup_origin_and_local();

            let mut origin_store = GitEventStore::new(origin_dir.path()).unwrap();
            origin_store
                .append(&make_event("from-origin", "from-origin-a1b2"))
                .unwrap();

            // Stamp the same schema version as local
            stamp_origin_schema_version(&origin_dir, CURRENT_SCHEMA_VERSION);

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            local_store.sync(&mut bus, &output).unwrap();

            let events = EventStore::get_all_events(&local_store).unwrap();
            assert_eq!(events.len(), 1, "Sync should succeed normally");
        }

        #[test]
        fn sync_succeeds_when_peer_has_no_schema_version() {
            let (_origin_dir, _local_dir, mut local_store) = setup_origin_and_local();

            // Origin has no events, no schema version at all
            // Just push a local event so there's something to sync
            local_store
                .append(&make_event("local-yak", "local-yak-a1b2"))
                .unwrap();

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            // Should succeed — no peer ref means no version conflict
            local_store.sync(&mut bus, &output).unwrap();
        }

        #[test]
        fn sync_migrates_peer_with_older_schema_version() {
            use crate::adapters::event_store::migration::read_schema_version;

            let (origin_dir, _local_dir, mut local_store) = setup_origin_and_local();

            // Create a v3-format event on origin (has name and id, but uses
            // nested tree structure which v3→v4 migration flattens).
            // This is a realistic scenario: origin written by a binary one
            // version behind.
            {
                let origin_repo = Repository::open(origin_dir.path()).unwrap();
                let state_blob = origin_repo.blob(b"todo").unwrap();
                let context_blob = origin_repo.blob(b"").unwrap();
                let name_blob = origin_repo.blob(b"make the tea").unwrap();
                let id_blob = origin_repo.blob(b"make-the-tea-a1b2").unwrap();

                let mut yak_builder = origin_repo.treebuilder(None).unwrap();
                yak_builder.insert("state", state_blob, 0o100644).unwrap();
                yak_builder
                    .insert("context.md", context_blob, 0o100644)
                    .unwrap();
                yak_builder.insert("name", name_blob, 0o100644).unwrap();
                yak_builder.insert("id", id_blob, 0o100644).unwrap();
                let yak_tree = yak_builder.write().unwrap();

                let version_blob = origin_repo.blob(b"3").unwrap();
                let mut root_builder = origin_repo.treebuilder(None).unwrap();
                root_builder
                    .insert("make-the-tea-a1b2", yak_tree, 0o040000)
                    .unwrap();
                root_builder
                    .insert(".schema-version", version_blob, 0o100644)
                    .unwrap();
                let root_tree_oid = root_builder.write().unwrap();
                let root_tree = origin_repo.find_tree(root_tree_oid).unwrap();

                let sig = origin_repo.signature().unwrap();
                origin_repo
                    .commit(
                        Some("refs/notes/yaks"),
                        &sig,
                        &sig,
                        "Added: \"make the tea\" \"make-the-tea-a1b2\"",
                        &root_tree,
                        &[],
                    )
                    .unwrap();

                // Verify origin is at v3
                let origin_location = EventStoreLocation {
                    repo: &origin_repo,
                    ref_name: "refs/notes/yaks",
                };
                assert_eq!(
                    read_schema_version(&origin_location).unwrap(),
                    Some(3),
                    "Origin should be at v3"
                );
            }

            let mut bus = EventBus::new();
            let (output, _) = make_test_display();

            // Sync should succeed — migrating the peer ref from v3 to v4
            local_store.sync(&mut bus, &output).unwrap();

            // Verify we got the event
            let events = EventStore::get_all_events(&local_store).unwrap();
            assert_eq!(events.len(), 1, "Should have pulled the event from origin");
        }
    }
}
