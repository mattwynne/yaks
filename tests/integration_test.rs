use anyhow::Result;
use serial_test::serial;
use std::env;
use tempfile::TempDir;
use yx::ports::{LogPort, StoragePort};

/// No-op log implementation for tests
struct NoOpLog;

impl LogPort for NoOpLog {
    fn log_command(&self, _command: &str) -> Result<()> {
        Ok(())
    }
}

/// Helper to run yx commands in a test environment
struct TestEnv {
    _temp_dir: TempDir,
    yak_path: String,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let yak_path = temp_dir.path().to_str().unwrap().to_string();

        // Prevent editor from opening in integration tests
        env::set_var("YX_IGNORE_STDIN", "1");

        Self {
            _temp_dir: temp_dir,
            yak_path,
        }
    }

    fn yak_exists(&self, name: &str) -> bool {
        let path = format!("{}/{}", self.yak_path, name);
        std::path::Path::new(&path).exists()
    }
}

#[test]
#[serial]
fn test_add_yak_creates_directory() {
    let test_env = TestEnv::new();

    // Set YAK_PATH for the test
    env::set_var("YAK_PATH", &test_env.yak_path);

    // Create DirectoryStorage and ConsoleOutput
    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Execute AddYak use case
    let use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    use_case.execute("integration-test-yak").unwrap();

    // Verify the yak directory was created
    assert!(test_env.yak_exists("integration-test-yak"));
}

#[test]
#[serial]
fn test_add_yak_can_be_retrieved() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("test-retrieval").unwrap();

    // Retrieve it using the storage port
    let yak = storage.get_yak("test-retrieval").unwrap();
    assert_eq!(yak.name, "test-retrieval");
    assert!(!yak.done);
}

#[test]
#[serial]
fn test_list_empty_yaks() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // List should succeed even with no yaks
    let list_use_case = yx::application::ListYaks::new(&storage, &output);
    list_use_case.execute("plain", None).unwrap();
}

#[test]
#[serial]
fn test_list_shows_added_yaks() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add some yaks
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("yak-one").unwrap();
    add_use_case.execute("yak-two").unwrap();

    // List them
    let list_use_case = yx::application::ListYaks::new(&storage, &output);
    list_use_case.execute("plain", None).unwrap();

    // Verify both yaks exist
    let yaks = storage.list_yaks().unwrap();
    assert_eq!(yaks.len(), 2);
    assert!(yaks.iter().any(|y| y.name == "yak-one"));
    assert!(yaks.iter().any(|y| y.name == "yak-two"));
}

#[test]
#[serial]
fn test_done_yak_marks_as_done() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("test-yak").unwrap();

    // Mark it as done
    let done_use_case = yx::application::DoneYak::new(&storage, &output, &NoOpLog);
    done_use_case.execute("test-yak", false, false).unwrap();

    // Verify it's marked as done
    let yak = storage.get_yak("test-yak").unwrap();
    assert!(yak.done);
}

#[test]
#[serial]
fn test_done_yak_with_undo_marks_as_not_done() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak and mark it done
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("test-yak").unwrap();
    let done_use_case = yx::application::DoneYak::new(&storage, &output, &NoOpLog);
    done_use_case.execute("test-yak", false, false).unwrap();

    // Verify it's marked as done
    let yak = storage.get_yak("test-yak").unwrap();
    assert!(yak.done);

    // Mark it as not done using undo flag
    done_use_case.execute("test-yak", true, false).unwrap();

    // Verify it's no longer marked as done
    let yak = storage.get_yak("test-yak").unwrap();
    assert!(!yak.done);
}

#[test]
#[serial]
fn test_done_yak_fails_for_nonexistent_yak() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Try to mark a non-existent yak as done
    let done_use_case = yx::application::DoneYak::new(&storage, &output, &NoOpLog);
    let result = done_use_case.execute("nonexistent", false, false);

    assert!(result.is_err());
}

#[test]
#[serial]
fn test_remove_yak_deletes_directory() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("test-yak").unwrap();

    // Verify it exists
    assert!(test_env.yak_exists("test-yak"));

    // Remove it
    let remove_use_case = yx::application::RemoveYak::new(&storage, &output, &NoOpLog);
    remove_use_case.execute("test-yak").unwrap();

    // Verify it no longer exists
    assert!(!test_env.yak_exists("test-yak"));
}

#[test]
#[serial]
fn test_remove_yak_fails_for_nonexistent_yak() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Try to remove a non-existent yak
    let remove_use_case = yx::application::RemoveYak::new(&storage, &output, &NoOpLog);
    let result = remove_use_case.execute("nonexistent");

    assert!(result.is_err());
}

#[test]
#[serial]
fn test_remove_yak_can_remove_done_yak() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak and mark it done
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("done-yak").unwrap();
    let done_use_case = yx::application::DoneYak::new(&storage, &output, &NoOpLog);
    done_use_case.execute("done-yak", false, false).unwrap();

    // Remove the done yak
    let remove_use_case = yx::application::RemoveYak::new(&storage, &output, &NoOpLog);
    remove_use_case.execute("done-yak").unwrap();

    // Verify it's gone
    assert!(!test_env.yak_exists("done-yak"));
}

#[test]
#[serial]
fn test_prune_removes_all_done_yaks() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add multiple yaks
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("done-yak-1").unwrap();
    add_use_case.execute("done-yak-2").unwrap();
    add_use_case.execute("active-yak").unwrap();

    // Mark some as done
    let done_use_case = yx::application::DoneYak::new(&storage, &output, &NoOpLog);
    done_use_case.execute("done-yak-1", false, false).unwrap();
    done_use_case.execute("done-yak-2", false, false).unwrap();

    // Prune done yaks
    let prune_use_case = yx::application::PruneYaks::new(&storage, &output, &NoOpLog);
    prune_use_case.execute().unwrap();

    // Verify done yaks are removed
    assert!(!test_env.yak_exists("done-yak-1"));
    assert!(!test_env.yak_exists("done-yak-2"));

    // Verify active yak still exists
    assert!(test_env.yak_exists("active-yak"));
}

#[test]
#[serial]
fn test_prune_handles_no_done_yaks() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add only active yaks
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("active-yak-1").unwrap();
    add_use_case.execute("active-yak-2").unwrap();

    // Prune (should handle gracefully)
    let prune_use_case = yx::application::PruneYaks::new(&storage, &output, &NoOpLog);
    prune_use_case.execute().unwrap();

    // Verify all yaks still exist
    assert!(test_env.yak_exists("active-yak-1"));
    assert!(test_env.yak_exists("active-yak-2"));
}

#[test]
#[serial]
fn test_prune_handles_empty_list() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Prune when no yaks exist (should handle gracefully)
    let prune_use_case = yx::application::PruneYaks::new(&storage, &output, &NoOpLog);
    prune_use_case.execute().unwrap();
}

#[test]
#[serial]
fn test_move_yak_renames_directory() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("old-name").unwrap();

    // Verify it exists
    assert!(test_env.yak_exists("old-name"));

    // Move it
    let move_use_case = yx::application::MoveYak::new(&storage, &output, &NoOpLog);
    move_use_case.execute("old-name", "new-name").unwrap();

    // Verify old name no longer exists and new name does
    assert!(!test_env.yak_exists("old-name"));
    assert!(test_env.yak_exists("new-name"));
}

#[test]
#[serial]
fn test_move_yak_preserves_done_status() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak and mark it done
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("done-yak").unwrap();
    let done_use_case = yx::application::DoneYak::new(&storage, &output, &NoOpLog);
    done_use_case.execute("done-yak", false, false).unwrap();

    // Move it
    let move_use_case = yx::application::MoveYak::new(&storage, &output, &NoOpLog);
    move_use_case
        .execute("done-yak", "renamed-done-yak")
        .unwrap();

    // Verify done status is preserved
    let yak = storage.get_yak("renamed-done-yak").unwrap();
    assert!(yak.done);
}

#[test]
#[serial]
fn test_move_yak_preserves_context() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak with context
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("yak-with-context").unwrap();
    storage
        .write_field(
            "yak-with-context",
            yx::domain::CONTEXT_FIELD,
            "Important context",
        )
        .unwrap();

    // Move it
    let move_use_case = yx::application::MoveYak::new(&storage, &output, &NoOpLog);
    move_use_case
        .execute("yak-with-context", "renamed-yak")
        .unwrap();

    // Verify context is preserved
    let context = storage
        .read_field("renamed-yak", yx::domain::CONTEXT_FIELD)
        .unwrap();
    assert_eq!(context, "Important context");
}

#[test]
#[serial]
fn test_move_yak_fails_for_nonexistent_yak() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Try to move a non-existent yak
    let move_use_case = yx::application::MoveYak::new(&storage, &output, &NoOpLog);
    let result = move_use_case.execute("nonexistent", "new-name");

    assert!(result.is_err());
}

#[test]
#[serial]
fn test_move_yak_fails_for_existing_target() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add two yaks
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("yak-1").unwrap();
    add_use_case.execute("yak-2").unwrap();

    // Move yak-1 to yak-2 (parent-only move - creates yak-2/yak-1)
    let move_use_case = yx::application::MoveYak::new(&storage, &output, &NoOpLog);
    let result = move_use_case.execute("yak-1", "yak-2");

    assert!(result.is_ok());
    assert!(storage.get_yak("yak-2/yak-1").is_ok());
}

#[test]
#[serial]
fn test_edit_context_fails_for_nonexistent_yak() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Try to edit context for a non-existent yak
    let edit_context_use_case = yx::application::EditContext::new(&storage, &output, &NoOpLog);
    let result = edit_context_use_case.execute("nonexistent");

    assert!(result.is_err());
}

// Note: Testing the full editor flow and stdin input is difficult in integration tests
// because it requires mocking stdin or spawning actual processes. The DirectoryStorage
// adapter's read_context and write_context methods are already tested, and the use case
// validation logic is tested in the unit tests. The editor and stdin handling would be
// tested through end-to-end tests or manual testing.

#[test]
#[serial]
fn test_show_context_fails_for_nonexistent_yak() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Try to show context for a non-existent yak
    let show_context_use_case = yx::application::ShowContext::new(&storage, &output);
    let result = show_context_use_case.execute("nonexistent");

    assert!(result.is_err());
}

#[test]
#[serial]
fn test_show_context_displays_empty_context() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak with no context
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("test-yak").unwrap();

    // Show context should succeed even with empty context
    let show_context_use_case = yx::application::ShowContext::new(&storage, &output);
    let result = show_context_use_case.execute("test-yak");

    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_show_context_displays_context_content() {
    let test_env = TestEnv::new();
    env::set_var("YAK_PATH", &test_env.yak_path);

    let storage = yx::adapters::storage::DirectoryStorage::new().unwrap();
    let output = yx::adapters::cli::ConsoleOutput;

    // Add a yak
    let add_use_case = yx::application::AddYak::new(&storage, &output, &NoOpLog);
    add_use_case.execute("test-yak").unwrap();

    // Write some context
    storage
        .write_field(
            "test-yak",
            yx::domain::CONTEXT_FIELD,
            "Test context content",
        )
        .unwrap();

    // Show context should succeed
    let show_context_use_case = yx::application::ShowContext::new(&storage, &output);
    let result = show_context_use_case.execute("test-yak");

    assert!(result.is_ok());

    // Verify context was written correctly
    let context = storage
        .read_field("test-yak", yx::domain::CONTEXT_FIELD)
        .unwrap();
    assert_eq!(context, "Test context content");
}

#[test]
fn test_default_format_is_pretty() {
    use std::process::Command;

    let tmp_dir = TempDir::new().unwrap();
    let yak_path = tmp_dir.path().to_str().unwrap();

    // Add a yak
    Command::new(env!("CARGO_BIN_EXE_yx"))
        .env("YAK_PATH", yak_path)
        .env("YX_IGNORE_STDIN", "1")
        .env("YX_SKIP_GIT_CHECKS", "1")
        .args(["add", "test-yak"])
        .output()
        .unwrap();

    // List without format flag
    let output = Command::new(env!("CARGO_BIN_EXE_yx"))
        .env("YAK_PATH", yak_path)
        .env("YX_IGNORE_STDIN", "1")
        .env("YX_SKIP_GIT_CHECKS", "1")
        .args(["ls"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Should have pretty format dot, not markdown checkbox
    assert!(stdout.contains("○"));
    assert!(!stdout.contains("[ ]"));
}
