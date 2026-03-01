// Use case: Reset git event store from disk projection
//
// Wipes the git event history and replays all yaks from the disk
// projection through the Application layer. This rebuilds a clean
// event stream from the current .yaks directory state.
//
// This is the `yx reset --git-from-disk` mode.

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::domain::slug::YakId;
use crate::domain::YakView;

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
                app.display.info("Aborted.");
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
        let yak_index: HashMap<&YakId, &YakView> = yaks.iter().map(|y| (&y.id, y)).collect();

        // Find roots: yaks not appearing in any other yak's children list
        let mut child_ids = HashSet::new();
        for yak in &yaks {
            for child_id in &yak.children {
                child_ids.insert(child_id);
            }
        }
        let roots: Vec<&YakView> = yaks.iter().filter(|y| !child_ids.contains(&y.id)).collect();

        for root_yak in &roots {
            replay_yak(app, root_yak, &yak_index, None)?;
        }

        // 5. Report results
        app.display
            .info(&format!("Reset from disk: {} yaks", yak_count));
        app.display.info("");
        app.display.info("To update the remote, run:");
        app.display
            .info("  git push origin refs/notes/yaks --force");
        app.display.info("");
        app.display.info("Collaborators must then run:");
        app.display
            .info("  git fetch origin refs/notes/yaks:refs/notes/yaks --force");
        Ok(())
    }
}

fn replay_yak(
    app: &mut Application,
    yak: &YakView,
    yak_index: &HashMap<&YakId, &YakView>,
    parent_id: Option<&str>,
) -> Result<()> {
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
    if yak.state != "todo" {
        use_case = use_case.with_state(Some(&yak.state));
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
        use_case = use_case.with_field("tags", &tag_content);
    }
    app.handle(use_case)?;

    for child_id in &yak.children {
        if let Some(child) = yak_index.get(child_id) {
            replay_yak(app, child, yak_index, Some(yak.id.as_str()))?;
        }
    }
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
        assert_eq!(yak.state, "wip");
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
}
