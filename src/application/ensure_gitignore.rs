// EnsureGitignore use case - handles first-time .yaks setup
//
// This use case ensures .yaks is gitignored, prompting interactively if needed.
// It orchestrates the onboarding flow through the LocalWorkspacePort.

use crate::application::{Application, UseCase};
use anyhow::Result;
use std::io::{self, Write};

/// Ensure .yaks is gitignored use case
///
/// This use case:
/// 1. Checks if .yaks is already gitignored
/// 2. If not, prompts interactively to add it
/// 3. Optionally commits the change
///
/// Returns Ok(()) if .yaks is gitignored (or was successfully added).
/// Returns Err if .yaks is not gitignored and cannot be added.
pub struct EnsureGitignore;

impl EnsureGitignore {
    pub fn new() -> Self {
        Self
    }

    /// Prompt user to add .yaks to .gitignore with [Y/n] default
    fn prompt_add_gitignore() -> Result<bool> {
        eprintln!();
        eprintln!("👋 It looks like you've never used yaks in this repo before!");
        eprintln!("   I need to add .yaks to your .gitignore to keep things tidy.");
        eprintln!();
        eprint!("   Add .yaks to .gitignore? [Y/n] ");
        io::stderr().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let add_to_gitignore =
            response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y");

        Ok(add_to_gitignore)
    }

    /// Prompt user to commit .gitignore with [Y/n] default
    fn prompt_commit() -> Result<bool> {
        eprintln!("   ✅ Added .yaks to .gitignore");
        eprintln!();
        eprint!("   Commit this change now? [Y/n] ");
        io::stderr().flush()?;

        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        let should_commit = response.trim().is_empty() || response.trim().eq_ignore_ascii_case("y");

        Ok(should_commit)
    }
}

impl Default for EnsureGitignore {
    fn default() -> Self {
        Self::new()
    }
}

impl UseCase for EnsureGitignore {
    fn execute(&self, app: &mut Application) -> Result<()> {
        // Check if .yaks is already gitignored
        if app.local_workspace.is_yaks_gitignored()? {
            return Ok(());
        }

        // Not gitignored - check if we can prompt
        if !app.input.is_interactive() {
            anyhow::bail!(".yaks is not gitignored. Fix with: echo '.yaks' >> .gitignore");
        }

        // Interactive mode: offer to add .yaks to .gitignore
        if !Self::prompt_add_gitignore()? {
            // User declined
            anyhow::bail!("Error: .yaks folder is not gitignored");
        }

        // Add .yaks to .gitignore
        app.local_workspace.add_yaks_to_gitignore()?;

        // Ask about committing
        if Self::prompt_commit()? {
            app.local_workspace.commit_gitignore()?;
            eprintln!("   ✅ Committed!");
        } else {
            eprintln!("   Please remember to commit .gitignore");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{make_test_display, InMemoryEventStore, InMemoryInput, InMemoryStorage};
    use crate::application::UseCase;
    use crate::domain::event_metadata::Author;
    use crate::domain::ports::{AuthenticationPort, LocalWorkspacePort};
    use crate::infrastructure::EventBus;

    struct TestAuth;

    impl AuthenticationPort for TestAuth {
        fn current_author(&self) -> Author {
            Author {
                name: "test".to_string(),
                email: "test@test.com".to_string(),
            }
        }
    }

    /// Workspace that reports .yaks as NOT gitignored
    struct NotGitignoredWorkspace;

    impl LocalWorkspacePort for NotGitignoredWorkspace {
        fn is_yaks_gitignored(&self) -> Result<bool> {
            Ok(false)
        }

        fn add_yaks_to_gitignore(&self) -> Result<()> {
            Ok(())
        }

        fn commit_gitignore(&self) -> Result<()> {
            Ok(())
        }

        fn is_agent_session(&self) -> bool {
            false
        }
    }

    #[test]
    fn non_interactive_bails_when_not_gitignored() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, _) = make_test_display();
        let mut input = InMemoryInput::new();
        input.set_interactive(false);
        let auth = TestAuth;
        let workspace = NotGitignoredWorkspace;

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            None,
            &auth,
        );

        let result = EnsureGitignore::new().execute(&mut app);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(".yaks is not gitignored"),);
    }

    #[test]
    fn interactive_does_not_bail_when_not_gitignored() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, _) = make_test_display();
        let input = InMemoryInput::new(); // interactive by default
        let auth = TestAuth;
        let workspace = NotGitignoredWorkspace;

        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            None,
            &auth,
        );

        let result = EnsureGitignore::new().execute(&mut app);
        // Should NOT get the non-interactive bail error.
        // It may succeed (stdin EOF defaults to Y) or fail
        // differently, but it must not contain the
        // non-interactive error message.
        match result {
            Ok(()) => {} // passed through interactive flow
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    !msg.contains(".yaks is not gitignored"),
                    "Interactive mode should not bail with \
                     non-interactive error, got: {msg}",
                );
            }
        }
    }
}
