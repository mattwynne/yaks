// Use case: Reset git event store from disk projection
//
// Wipes the git event history and replays all yaks from the disk
// projection through the Application layer. This rebuilds a clean
// event stream from the current .yaks directory state.
//
// This is the `yx reset --git-from-disk` mode.

use std::collections::HashMap;

use crate::adapters::views::Message;
use anyhow::Result;

use crate::domain::slug::YakId;
use crate::domain::Yak;

use super::{AddYak, Application, UseCase};

#[derive(Default)]
pub struct ResetGitFromDisk {
    force: bool,
}

impl ResetGitFromDisk {
    pub fn new() -> Self {
        Self::default()
    }

    /// Skip the confirmation prompt (equivalent to --force flag)
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

impl UseCase for ResetGitFromDisk {
    fn execute(&self, app: &mut Application) -> Result<()> {
        if !self.force {
            let confirmed = app
                .input
                .confirm("This will wipe the git event log and rebuild from disk. Continue?")?;
            if !confirmed {
                app.display.message(&Message::Info("Aborted.".into()));
                return Ok(());
            }
        }

        // 1. Read all yaks from the current disk projection
        let yaks = app.store.list_yaks()?;
        let yak_count = yaks.len();

        // 2. Wipe the event store (deletes git ref)
        app.event_store.wipe()?;

        // 3. Clear disk storage via event bus rebuild with empty events
        app.event_bus.rebuild(&[])?;

        // 4. Replay yaks through AddYak in topological order (parents before children)
        replay_yaks_in_order(app, &yaks)?;

        // 5. Report results
        app.display.message(&Message::Info(format!(
            "Reset from disk: {} yaks",
            yak_count
        )));
        app.display.message(&Message::Info(String::new()));
        app.display
            .message(&Message::Info("To update the remote, run:".into()));
        app.display.message(&Message::Info(
            "  git push origin refs/notes/yaks --force".into(),
        ));
        app.display.message(&Message::Info(String::new()));
        app.display
            .message(&Message::Info("Collaborators must then run:".into()));
        app.display.message(&Message::Info(
            "  git fetch origin refs/notes/yaks:refs/notes/yaks --force".into(),
        ));
        Ok(())
    }
}

/// Replay all yaks in topological order (parents before children) using an iterative approach.
/// This avoids stack overflow and eliminates the timeout mutant that could occur with
/// recursive traversal when the parent-child filter is mutated.
fn replay_yaks_in_order(app: &mut Application, yaks: &[Yak]) -> Result<()> {
    // Build children lookup: parent_id -> Vec<&Yak>
    // This pre-computes the parent-child relationships ONCE using ==
    // If mutated to !=, it builds a wrong map but won't cause infinite loops
    let mut children_of: HashMap<Option<&YakId>, Vec<&Yak>> = HashMap::new();
    for yak in yaks {
        children_of
            .entry(yak.parent_id.as_ref())
            .or_default()
            .push(yak);
    }

    // Use a stack for depth-first traversal (iterative, not recursive)
    // Each entry is (yak, parent_id_str)
    let mut stack: Vec<(&Yak, Option<&str>)> = Vec::new();

    // Push roots (no parent) onto stack in reverse order so they process in order
    if let Some(roots) = children_of.get(&None) {
        for root in roots.iter().rev() {
            stack.push((root, None));
        }
    }

    // Process the stack
    while let Some((yak, parent_id)) = stack.pop() {
        // Replay this single yak
        replay_single_yak(app, yak, parent_id)?;

        // Push children onto stack in reverse order so they process in order
        if let Some(kids) = children_of.get(&Some(&yak.id)) {
            for child in kids.iter().rev() {
                stack.push((child, Some(yak.id.as_str())));
            }
        }
    }

    Ok(())
}

/// Replay a single yak through the AddYak use case, preserving all its properties.
fn replay_single_yak(app: &mut Application, yak: &Yak, parent_id: Option<&str>) -> Result<()> {
    let has_real_metadata = yak.created_at != crate::domain::Timestamp::zero();
    let mut use_case = AddYak::new(yak.name.as_str())
        .with_id(Some(yak.id.as_str()))
        .with_context(yak.context.as_deref())
        .with_author(if has_real_metadata {
            Some(yak.created_by.clone())
        } else {
            None
        })
        .with_timestamp(if has_real_metadata {
            Some(yak.created_at)
        } else {
            None
        });
    if yak.state != crate::domain::YakState::Todo {
        let state_str = yak.state.to_string();
        use_case = use_case.with_state(Some(&state_str));
    }
    if let Some(pid) = parent_id {
        use_case = use_case.with_parent(Some(pid));
    }
    for (key, value) in &yak.fields {
        use_case = use_case.with_field(key, value);
    }
    if !yak.tags.is_empty() {
        let tag_content = yak.tags.join(
            "
",
        );
        use_case = use_case.with_field(crate::domain::field::TAGS_FIELD, &tag_content);
    }
    app.handle(use_case)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::AddYak;
    use crate::domain::ports::{EventStore, ReadYakStore};
    use crate::domain::slug::YakId;
    use crate::infrastructure::EventBus;

    #[test]
    fn reset_git_from_disk_replays_yaks_to_event_store() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, output) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            // Add some yaks
            app.handle(AddYak::new("parent-yak")).unwrap();
            app.handle(AddYak::new("child-yak").with_parent(Some("parent-yak")))
                .unwrap();
        }

        // Verify events were created
        let original_event_count = EventStore::get_all_events(&event_store).unwrap().len();
        assert!(original_event_count > 0);

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            // Reset git from disk
            app.handle(ResetGitFromDisk::new()).unwrap();
        }

        // Yaks should still exist in storage
        assert!(ReadYakStore::get_yak(&storage, &YakId::from("parent-yak")).is_ok());
        assert!(ReadYakStore::fuzzy_find_yak_id(&storage, "child-yak").is_ok());

        // Event store should have new events (from replay)
        let new_events = EventStore::get_all_events(&event_store).unwrap();
        assert!(!new_events.is_empty());

        // Output should contain the reset message
        let output_text = output.contents();
        assert!(
            output_text.contains("Reset from disk: 2 yaks"),
            "Expected reset message in output, got: {}",
            output_text
        );
    }

    #[test]
    fn reset_git_from_disk_preserves_state() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            // Add a yak with non-default state
            app.handle(AddYak::new("wip-yak").with_state(Some("wip")))
                .unwrap();
        }

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            // Reset git from disk
            app.handle(ResetGitFromDisk::new()).unwrap();
        }

        // State should be preserved
        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "wip-yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert_eq!(yak.state, crate::domain::YakState::Wip);
    }

    #[test]
    fn reset_git_from_disk_works_with_empty_store() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, output) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            None,
            &auth,
        );

        // Reset with no yaks should succeed
        app.handle(ResetGitFromDisk::new()).unwrap();

        let output_text = output.contents();
        assert!(
            output_text.contains("Reset from disk: 0 yaks"),
            "Expected empty reset message, got: {}",
            output_text
        );
    }
    #[test]
    fn reset_git_from_disk_aborts_when_not_confirmed() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, output) = make_test_display();
        let input = InMemoryInput::new();
        input.set_confirm(false);
        let auth = InMemoryAuthentication::new();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            app.handle(AddYak::new("test-yak")).unwrap();
        }

        let events_before = EventStore::get_all_events(&event_store).unwrap().len();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            // Reset should be a no-op when user declines
            app.handle(ResetGitFromDisk::new()).unwrap();
        }

        // Event store should be unchanged
        let events_after = EventStore::get_all_events(&event_store).unwrap().len();
        assert_eq!(
            events_before, events_after,
            "Events should not change when user declines"
        );

        // Output should contain abort message
        let output_text = output.contents();
        assert!(
            output_text.contains("Aborted"),
            "Expected 'Aborted' in output, got: {}",
            output_text
        );
    }

    #[test]
    fn reset_git_from_disk_with_force_skips_confirmation() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, output) = make_test_display();
        let input = InMemoryInput::new();
        input.set_confirm(false); // Would decline, but --force overrides
        let auth = InMemoryAuthentication::new();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            app.handle(AddYak::new("test-yak")).unwrap();
        }

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            // Force should skip confirmation
            app.handle(ResetGitFromDisk::new().with_force(true))
                .unwrap();
        }

        // Output should contain the reset message, not abort
        let output_text = output.contents();
        assert!(
            output_text.contains("Reset from disk: 1 yaks"),
            "Expected reset message in output, got: {}",
            output_text
        );
    }

    #[test]
    fn reset_git_from_disk_preserves_author_and_timestamp() {
        use crate::domain::event_metadata::{Author, Timestamp};
        use crate::domain::ports::EventStore;

        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        let custom_author = Author {
            name: "Jane Doe".to_string(),
            email: "jane@example.com".to_string(),
        };
        let custom_timestamp = Timestamp(1700000000);

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            app.handle(
                AddYak::new("authored-yak")
                    .with_author(Some(custom_author.clone()))
                    .with_timestamp(Some(custom_timestamp)),
            )
            .unwrap();
        }

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            app.handle(ResetGitFromDisk::new()).unwrap();
        }

        // Verify author and timestamp are preserved by checking the replayed events
        let events = EventStore::get_all_events(&event_store).unwrap();
        assert!(
            !events.is_empty(),
            "Expected at least one event after reset"
        );
        let first_event = &events[0];
        assert_eq!(
            first_event.metadata().author,
            custom_author,
            "Author should be preserved after reset-from-disk"
        );
        assert_eq!(
            first_event.metadata().timestamp,
            custom_timestamp,
            "Timestamp should be preserved after reset-from-disk"
        );
    }

    #[test]
    fn reset_git_from_disk_preserves_tags() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            app.handle(AddYak::new("tagged-yak")).unwrap();
            app.handle(crate::application::AddTag::new(
                "tagged-yak",
                vec!["urgent".to_string(), "backend".to_string()],
            ))
            .unwrap();
        }

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            app.handle(ResetGitFromDisk::new()).unwrap();
        }

        // Verify tags are preserved
        let id = ReadYakStore::fuzzy_find_yak_id(&storage, "tagged-yak").unwrap();
        let yak = ReadYakStore::get_yak(&storage, &id).unwrap();
        assert!(
            yak.tags.contains(&"urgent".to_string()),
            "Expected 'urgent' tag, got: {:?}",
            yak.tags
        );
        assert!(
            yak.tags.contains(&"backend".to_string()),
            "Expected 'backend' tag, got: {:?}",
            yak.tags
        );
    }

    #[test]
    fn reset_git_from_disk_replays_children_under_correct_parents() {
        // This test kills the mutant that changes == to != in the children filter.
        // Setup: one root with two children. With the != mutant, children won't be
        // replayed at all (filter would select nothing), causing this test to fail
        // quickly when trying to look them up.
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();

        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));

        let (display, _) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            // Create one parent with two children
            app.handle(AddYak::new("parent")).unwrap();
            app.handle(AddYak::new("child-a").with_parent(Some("parent")))
                .unwrap();
            app.handle(AddYak::new("child-b").with_parent(Some("parent")))
                .unwrap();
        }

        {
            let mut app = Application::new(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                None,
                &auth,
            );

            // Reset git from disk
            app.handle(ResetGitFromDisk::new()).unwrap();
        }

        // Verify children exist and have correct parent
        // With != mutation, children won't be replayed, so these lookups will fail
        let parent_id = ReadYakStore::fuzzy_find_yak_id(&storage, "parent").unwrap();

        let child_a_id = ReadYakStore::fuzzy_find_yak_id(&storage, "child-a").unwrap();
        let child_a = ReadYakStore::get_yak(&storage, &child_a_id).unwrap();
        assert_eq!(
            child_a.parent_id.as_ref(),
            Some(&parent_id),
            "child-a should be under parent"
        );

        let child_b_id = ReadYakStore::fuzzy_find_yak_id(&storage, "child-b").unwrap();
        let child_b = ReadYakStore::get_yak(&storage, &child_b_id).unwrap();
        assert_eq!(
            child_b.parent_id.as_ref(),
            Some(&parent_id),
            "child-b should be under parent"
        );
    }
}
