// Step definitions using TestWorld trait
//
// These steps work with both FullStackWorld and InProcessWorld
// through the TestWorld trait interface.

use anyhow::{Context, Result};
use cucumber::{given, then, when};

use super::full_stack_world::FullStackWorld;
use super::in_process_world::InProcessWorld;
use super::test_world::{strip_ansi_codes, TestWorld};

// ============================================================================
// Given steps
// ============================================================================

#[given(expr = "I have a clean git repository")]
async fn clean_git_repo_full_stack(world: &mut FullStackWorld) -> Result<()> {
    world.init_git()
}

#[given(expr = "I have a clean git repository")]
async fn clean_git_repo_in_process(_world: &mut InProcessWorld) -> Result<()> {
    // No git needed in in-process mode
    Ok(())
}

#[given(regex = r#"^I add the yak "([^"]+)"$"#)]
async fn add_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

#[given(regex = r#"^I add the yak "([^"]+)"$"#)]
async fn add_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

#[given(regex = r#"^I add the yak "([^"]+)" blocking "([^"]+)"$"#)]
async fn add_yak_blocking_full_stack(world: &mut FullStackWorld, yak_name: String, parent: String) -> Result<()> {
    world.add_yak_blocking(&yak_name, &parent)
}

#[given(regex = r#"^I add the yak "([^"]+)" blocking "([^"]+)"$"#)]
async fn add_yak_blocking_in_process(world: &mut InProcessWorld, yak_name: String, parent: String) -> Result<()> {
    world.add_yak_blocking(&yak_name, &parent)
}

#[when(regex = r#"^I add the yak "([^"]+)" blocking "([^"]+)"$"#)]
async fn when_add_yak_blocking_full_stack(world: &mut FullStackWorld, yak_name: String, parent: String) -> Result<()> {
    world.add_yak_blocking(&yak_name, &parent)
}

#[when(regex = r#"^I add the yak "([^"]+)" blocking "([^"]+)"$"#)]
async fn when_add_yak_blocking_in_process(world: &mut InProcessWorld, yak_name: String, parent: String) -> Result<()> {
    world.add_yak_blocking(&yak_name, &parent)
}

#[given(regex = r#"^I mark the yak "(.+)" as done$"#)]
async fn done_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.done_yak(&yak_name)
}

#[given(regex = r#"^I mark the yak "(.+)" as done$"#)]
async fn done_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.done_yak(&yak_name)
}

// ============================================================================
// V1 schema fixture (inline duplication of the v1 event store format)
// ============================================================================

/// Create a yak directly in the git event store using the v1 schema format.
/// This is intentionally duplicated/inlined — it's a frozen snapshot of how
/// v1 works, so that when the production code evolves, this fixture still
/// creates the old format to prove migration works.
#[given(regex = r#"^a yak "(.+)" created with the v1 schema$"#)]
async fn v1_yak(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.init_git()?;
    let repo_path = world.default_repo_path();

    // -- Build the event store commit on refs/notes/yaks --

    // Create blobs for yak files
    let state_oid = git_hash_object(repo_path, "todo")?;
    let context_oid = git_hash_object(repo_path, "")?;

    // Create yak subtree: state + context.md
    let yak_tree_input = format!(
        "100644 blob {}\tstate\n100644 blob {}\tcontext.md\n",
        state_oid, context_oid
    );
    let yak_tree_oid = git_mktree(repo_path, &yak_tree_input)?;

    // Create root tree containing the yak subtree
    let root_tree_input = format!("040000 tree {}\t{}\n", yak_tree_oid, yak_name);
    let root_tree_oid = git_mktree(repo_path, &root_tree_input)?;

    // Create commit on refs/notes/yaks
    let message = format!("Added: \"{}\"", yak_name);
    let commit_oid = git_commit_tree(repo_path, &root_tree_oid, &message, None)?;
    git_update_ref(repo_path, "refs/notes/yaks", &commit_oid)?;

    // -- Build the .yaks/ projection (YAK_PATH = repo_path in tests) --
    let yak_dir = repo_path.join(&yak_name);
    std::fs::create_dir_all(&yak_dir).context("Failed to create yak directory")?;
    std::fs::write(yak_dir.join("state"), "todo").context("Failed to write state")?;
    std::fs::write(yak_dir.join("context.md"), "").context("Failed to write context.md")?;

    Ok(())
}

// -- Git plumbing helpers for v1 fixture --

fn git_hash_object(repo_path: &std::path::Path, content: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["hash-object", "-w", "--stdin"])
        .current_dir(repo_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(content.as_bytes())?;
            child.wait_with_output()
        })
        .context("git hash-object failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_mktree(repo_path: &std::path::Path, input: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["mktree"])
        .current_dir(repo_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(input.as_bytes())?;
            child.wait_with_output()
        })
        .context("git mktree failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_commit_tree(
    repo_path: &std::path::Path,
    tree_oid: &str,
    message: &str,
    parent: Option<&str>,
) -> Result<String> {
    let mut args = vec!["commit-tree", tree_oid, "-m", message];
    if let Some(parent_oid) = parent {
        args.extend(["-p", parent_oid]);
    }
    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output()
        .context("git commit-tree failed")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_update_ref(repo_path: &std::path::Path, ref_name: &str, oid: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["update-ref", ref_name, oid])
        .current_dir(repo_path)
        .status()
        .context("git update-ref failed")?;
    if !status.success() {
        anyhow::bail!("git update-ref failed");
    }
    Ok(())
}

// ============================================================================
// When steps
// ============================================================================

#[when(expr = "I list the yaks")]
async fn list_yaks_full_stack(world: &mut FullStackWorld) -> Result<()> {
    world.list_yaks()
}

#[when(expr = "I list the yaks")]
async fn list_yaks_in_process(world: &mut InProcessWorld) -> Result<()> {
    world.list_yaks()
}

#[when(regex = r#"^I list the yaks in "(.+)" format$"#)]
async fn list_yaks_format_full_stack(world: &mut FullStackWorld, format: String) -> Result<()> {
    world.list_yaks_with_format(&format)
}

#[when(regex = r#"^I list the yaks in "(.+)" format$"#)]
async fn list_yaks_format_in_process(world: &mut InProcessWorld, format: String) -> Result<()> {
    world.list_yaks_with_format(&format)
}

#[when(regex = r#"^I list the yaks in "(.+)" format filtering by "(.+)"$"#)]
async fn list_yaks_format_filter_full_stack(
    world: &mut FullStackWorld,
    format: String,
    only: String,
) -> Result<()> {
    world.list_yaks_with_format_and_filter(&format, &only)
}

#[when(regex = r#"^I list the yaks in "(.+)" format filtering by "(.+)"$"#)]
async fn list_yaks_format_filter_in_process(
    world: &mut InProcessWorld,
    format: String,
    only: String,
) -> Result<()> {
    world.list_yaks_with_format_and_filter(&format, &only)
}

#[when(regex = r#"^I add the yak "([^"]+)"$"#)]
async fn when_add_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

#[when(regex = r#"^I add the yak "([^"]+)"$"#)]
async fn when_add_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.add_yak(&yak_name)
}

#[when(regex = r#"^there should be (\d+) yaks?$"#)]
async fn yak_count_full_stack(world: &mut FullStackWorld, expected: usize) -> Result<()> {
    check_yak_count(world, expected)
}

#[when(regex = r#"^there should be (\d+) yaks?$"#)]
async fn yak_count_in_process(world: &mut InProcessWorld, expected: usize) -> Result<()> {
    check_yak_count(world, expected)
}

#[when(regex = r#"^I try to add the yak "([^"]+)"$"#)]
async fn try_add_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.try_add_yak(&yak_name)
}

#[when(regex = r#"^I try to add the yak "([^"]+)"$"#)]
async fn try_add_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.try_add_yak(&yak_name)
}

#[when(regex = r#"^I try to add the yak "([^"]+)" blocking "([^"]+)"$"#)]
async fn try_add_yak_blocking_full_stack(world: &mut FullStackWorld, yak_name: String, parent: String) -> Result<()> {
    world.try_add_yak_blocking(&yak_name, &parent)
}

#[when(regex = r#"^I try to add the yak "([^"]+)" blocking "([^"]+)"$"#)]
async fn try_add_yak_blocking_in_process(world: &mut InProcessWorld, yak_name: String, parent: String) -> Result<()> {
    world.try_add_yak_blocking(&yak_name, &parent)
}

#[when(regex = r#"^I remove the yak "(.+)"$"#)]
async fn remove_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.remove_yak(&yak_name)
}

#[when(regex = r#"^I remove the yak "(.+)"$"#)]
async fn remove_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.remove_yak(&yak_name)
}

#[when(regex = r#"^I try to remove the yak "(.+)"$"#)]
async fn try_remove_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.try_remove_yak(&yak_name)
}

#[when(regex = r#"^I try to remove the yak "(.+)"$"#)]
async fn try_remove_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.try_remove_yak(&yak_name)
}

#[when(regex = r#"^I mark the yak "(.+)" as done$"#)]
async fn when_done_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.done_yak(&yak_name)
}

#[when(regex = r#"^I mark the yak "(.+)" as done$"#)]
async fn when_done_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.done_yak(&yak_name)
}

#[when(regex = r#"^I try to mark the yak "(.+)" as done$"#)]
async fn try_done_yak_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.try_done_yak(&yak_name)
}

#[when(regex = r#"^I try to mark the yak "(.+)" as done$"#)]
async fn try_done_yak_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.try_done_yak(&yak_name)
}

#[when(regex = r#"^I mark the yak "(.+)" as done recursively$"#)]
async fn done_yak_recursive_full_stack(world: &mut FullStackWorld, yak_name: String) -> Result<()> {
    world.done_yak_recursive(&yak_name)
}

#[when(regex = r#"^I mark the yak "(.+)" as done recursively$"#)]
async fn done_yak_recursive_in_process(world: &mut InProcessWorld, yak_name: String) -> Result<()> {
    world.done_yak_recursive(&yak_name)
}

#[when(regex = r#"^I set the context of "(.+)" to "(.+)"$"#)]
async fn set_context_full_stack(
    world: &mut FullStackWorld,
    name: String,
    content: String,
) -> Result<()> {
    world.set_context(&name, &content)
}

#[when(regex = r#"^I set the context of "(.+)" to "(.+)"$"#)]
async fn set_context_in_process(
    world: &mut InProcessWorld,
    name: String,
    content: String,
) -> Result<()> {
    world.set_context(&name, &content)
}

#[when(regex = r#"^I show the context of "(.+)"$"#)]
async fn show_context_full_stack(world: &mut FullStackWorld, name: String) -> Result<()> {
    world.show_context(&name)
}

#[when(regex = r#"^I show the context of "(.+)"$"#)]
async fn show_context_in_process(world: &mut InProcessWorld, name: String) -> Result<()> {
    world.show_context(&name)
}

#[when(expr = "I prune done yaks")]
async fn prune_done_yaks_full_stack(world: &mut FullStackWorld) -> Result<()> {
    world.prune_yaks()
}

#[when(expr = "I prune done yaks")]
async fn prune_done_yaks_in_process(world: &mut InProcessWorld) -> Result<()> {
    world.prune_yaks()
}

#[when(regex = r#"^I set the state of "(.+)" to "(.+)"$"#)]
async fn set_state_full_stack(
    world: &mut FullStackWorld,
    name: String,
    state: String,
) -> Result<()> {
    world.set_state(&name, &state)
}

#[when(regex = r#"^I set the state of "(.+)" to "(.+)"$"#)]
async fn set_state_in_process(
    world: &mut InProcessWorld,
    name: String,
    state: String,
) -> Result<()> {
    world.set_state(&name, &state)
}

#[when(regex = r#"^I try to set the state of "(.+)" to "(.+)"$"#)]
async fn try_set_state_full_stack(
    world: &mut FullStackWorld,
    name: String,
    state: String,
) -> Result<()> {
    world.try_set_state(&name, &state)
}

#[when(regex = r#"^I try to set the state of "(.+)" to "(.+)"$"#)]
async fn try_set_state_in_process(
    world: &mut InProcessWorld,
    name: String,
    state: String,
) -> Result<()> {
    world.try_set_state(&name, &state)
}

#[when(regex = r#"^I start "(.+)"$"#)]
async fn start_yak_full_stack(world: &mut FullStackWorld, name: String) -> Result<()> {
    world.start_yak(&name)
}

#[when(regex = r#"^I start "(.+)"$"#)]
async fn start_yak_in_process(world: &mut InProcessWorld, name: String) -> Result<()> {
    world.start_yak(&name)
}

#[when(regex = r#"^I move the yak "(.+)" to "(.+)"$"#)]
async fn move_yak_full_stack(world: &mut FullStackWorld, from: String, to: String) -> Result<()> {
    world.move_yak(&from, &to)
}

#[when(regex = r#"^I move the yak "(.+)" to "(.+)"$"#)]
async fn move_yak_in_process(world: &mut InProcessWorld, from: String, to: String) -> Result<()> {
    world.move_yak(&from, &to)
}

#[when(regex = r#"^I set the "(.+)" field of "(.+)" to "(.+)"$"#)]
async fn set_field_full_stack(
    world: &mut FullStackWorld,
    field: String,
    name: String,
    content: String,
) -> Result<()> {
    world.set_field(&name, &field, &content)
}

#[when(regex = r#"^I set the "(.+)" field of "(.+)" to "(.+)"$"#)]
async fn set_field_in_process(
    world: &mut InProcessWorld,
    field: String,
    name: String,
    content: String,
) -> Result<()> {
    world.set_field(&name, &field, &content)
}

#[when(regex = r#"^I try to set the "(.+)" field of "(.+)" to "(.+)"$"#)]
async fn try_set_field_full_stack(
    world: &mut FullStackWorld,
    field: String,
    name: String,
    content: String,
) -> Result<()> {
    world.try_set_field(&name, &field, &content)
}

#[when(regex = r#"^I try to set the "(.+)" field of "(.+)" to "(.+)"$"#)]
async fn try_set_field_in_process(
    world: &mut InProcessWorld,
    field: String,
    name: String,
    content: String,
) -> Result<()> {
    world.try_set_field(&name, &field, &content)
}

#[when(regex = r#"^I show the "(.+)" field of "(.+)"$"#)]
async fn show_field_full_stack(
    world: &mut FullStackWorld,
    field: String,
    name: String,
) -> Result<()> {
    world.show_field(&name, &field)
}

#[when(regex = r#"^I show the "(.+)" field of "(.+)"$"#)]
async fn show_field_in_process(
    world: &mut InProcessWorld,
    field: String,
    name: String,
) -> Result<()> {
    world.show_field(&name, &field)
}

// ============================================================================
// Full-stack-only steps (CLI behavior that can't be tested in-process)
// ============================================================================

#[given(expr = "a directory that is not a git repository")]
async fn dir_not_git_repo(world: &mut FullStackWorld) -> Result<()> {
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    world.override_dir = Some(temp_dir);
    Ok(())
}

#[given(expr = "a git repository without .yaks in .gitignore")]
async fn git_repo_without_gitignore(world: &mut FullStackWorld) -> Result<()> {
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let status = std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .current_dir(temp_dir.path())
        .status()
        .context("Failed to run git init")?;
    if !status.success() {
        anyhow::bail!("git init failed");
    }
    world.override_dir = Some(temp_dir);
    Ok(())
}

#[when(expr = "I try to list the yaks from this directory")]
async fn list_yaks_in_override_dir(world: &mut FullStackWorld) -> Result<()> {
    world.run_yx_in_override_dir(&["ls"])
}

#[when(regex = r#"^I run yx (.+)$"#)]
async fn run_yx_raw_full_stack(world: &mut FullStackWorld, args: String) -> Result<()> {
    let parsed = shell_split(&args);
    let arg_vec: Vec<&str> = parsed.iter().map(|s| s.as_str()).collect();
    world.run_raw(&arg_vec)
}

#[when(regex = r#"^I add the yak "(.+)" with context "(.+)" from stdin$"#)]
async fn add_yak_with_stdin_full_stack(
    world: &mut FullStackWorld,
    yak_name: String,
    context: String,
) -> Result<()> {
    world.add_yak_with_stdin(&yak_name, &context)
}

#[when(regex = r#"^I set the context of "(.+)" from a file containing "(.+)"$"#)]
async fn set_context_from_file(
    world: &mut FullStackWorld,
    name: String,
    content: String,
) -> Result<()> {
    world.run_yx_with_file_stdin(&["context", &name], &content)
}

#[when(regex = r#"^I try to set the context of "(.+)" with empty stdin$"#)]
async fn try_set_context_empty_stdin(world: &mut FullStackWorld, name: String) -> Result<()> {
    world.run_yx_with_empty_stdin(&["context", &name])
}

#[when(regex = r#"^I try to set the "(.+)" field of "(.+)" with empty stdin$"#)]
async fn try_set_field_empty_stdin(
    world: &mut FullStackWorld,
    field: String,
    name: String,
) -> Result<()> {
    world.run_yx_with_empty_stdin(&["field", &name, &field])
}

#[when(regex = r#"^I invoke bash completion for words: (.+)$"#)]
async fn invoke_bash_completion(world: &mut FullStackWorld, words_str: String) -> Result<()> {
    world.run_bash_completion(&words_str)
}

#[then(regex = r#"^the completions should include "(.+)"$"#)]
async fn completions_should_include(world: &mut FullStackWorld, expected: String) -> Result<()> {
    check_output_includes(world, &expected)
}

// ============================================================================
// Multi-repo steps (sync tests)
// ============================================================================

#[given(regex = r#"^a bare git repository called "(.+)"$"#)]
async fn bare_git_repo(world: &mut FullStackWorld, name: String) -> Result<()> {
    world.create_bare_repo(&name)
}

#[given(regex = r#"^a git clone of "(.+)" called "(.+)"$"#)]
async fn git_clone(world: &mut FullStackWorld, origin: String, clone: String) -> Result<()> {
    world.create_clone(&origin, &clone)
}

#[given(regex = r#"^a git worktree of "(.+)" called "(.+)"$"#)]
async fn git_worktree(world: &mut FullStackWorld, parent: String, worktree: String) -> Result<()> {
    world.create_worktree(&parent, &worktree)
}

#[given(regex = r#"^"(.+)" has a yak called "(.+)"$"#)]
async fn repo_has_yak(world: &mut FullStackWorld, repo: String, yak: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["add", &yak])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to add yak '{}' in repo '{}':\nstdout: {}\nstderr: {}",
            yak,
            repo,
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[given(regex = r#"^"(.+)" has synced yaks$"#)]
async fn repo_has_synced(world: &mut FullStackWorld, repo: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["sync"])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to sync yaks in repo '{}':\nstdout: {}\nstderr: {}",
            repo,
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[when(regex = r#"^"(.+)" syncs yaks$"#)]
async fn repo_syncs_yaks(world: &mut FullStackWorld, repo: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["sync"])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Failed to sync yaks in repo '{}':\nstdout: {}\nstderr: {}",
            repo,
            world.get_output(),
            world.get_error()
        );
    }
    Ok(())
}

#[then(regex = r#"^"(.+)" has a "(.+)" ref$"#)]
async fn repo_has_ref(world: &mut FullStackWorld, repo: String, ref_name: String) -> Result<()> {
    world.run_git_in_repo(&repo, &["show-ref", &ref_name])?;
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Expected repo '{}' to have ref '{}', but show-ref failed",
            repo,
            ref_name
        );
    }
    Ok(())
}

#[then(regex = r#"^"(.+)" should have a yak called "(.+)"$"#)]
async fn repo_should_have_yak(world: &mut FullStackWorld, repo: String, yak: String) -> Result<()> {
    world.run_yx_in_repo(&repo, &["ls", "--format", "markdown"])?;
    let output = world.get_output();
    if !output.contains(&yak) {
        anyhow::bail!(
            "Expected repo '{}' to have yak '{}', but output was:\n{}",
            repo,
            yak,
            output
        );
    }
    Ok(())
}

#[then(regex = r#"^"(.+)" has nothing staged in the git index$"#)]
async fn repo_has_clean_index(world: &mut FullStackWorld, repo: String) -> Result<()> {
    world.run_git_in_repo(&repo, &["diff", "--cached", "--name-only"])?;
    let output = world.get_output();
    let trimmed = output.trim();
    if !trimmed.is_empty() {
        anyhow::bail!(
            "Expected repo '{}' to have nothing staged, but found:\n{}",
            repo,
            trimmed
        );
    }
    Ok(())
}

#[then(expr = "it should succeed")]
async fn should_succeed_full_stack(world: &mut FullStackWorld) -> Result<()> {
    check_should_succeed(world)
}

#[then(regex = r#"^the output should include "(.+)"$"#)]
async fn output_includes_full_stack(world: &mut FullStackWorld, expected: String) -> Result<()> {
    check_output_includes(world, &expected)
}

#[then(regex = r#"^the output should not include "(.+)"$"#)]
async fn output_not_includes_full_stack(
    world: &mut FullStackWorld,
    expected: String,
) -> Result<()> {
    check_output_not_includes(world, &expected)
}

#[then(regex = r#"^line (\d+) of the output should include "(.+)"$"#)]
async fn line_of_output_includes_full_stack(
    world: &mut FullStackWorld,
    line_num: usize,
    expected: String,
) -> Result<()> {
    check_line_of_output_includes(world, line_num, &expected)
}

// ============================================================================
// Then steps
// ============================================================================

#[then(expr = "the command should fail")]
async fn command_fails_full_stack(world: &mut FullStackWorld) -> Result<()> {
    check_command_fails(world)
}

#[then(expr = "the command should fail")]
async fn command_fails_in_process(world: &mut InProcessWorld) -> Result<()> {
    check_command_fails(world)
}

#[then(regex = r#"^the error should contain "(.+)"$"#)]
async fn error_contains_full_stack(world: &mut FullStackWorld, expected: String) -> Result<()> {
    check_error_contains(world, &expected)
}

#[then(regex = r#"^the error should contain "(.+)"$"#)]
async fn error_contains_in_process(world: &mut InProcessWorld, expected: String) -> Result<()> {
    check_error_contains(world, &expected)
}

#[then(expr = "the output should be:")]
async fn output_should_be_full_stack(
    world: &mut FullStackWorld,
    step: &cucumber::gherkin::Step,
) -> Result<()> {
    check_output(world, step)
}

#[then(expr = "the output should be:")]
async fn output_should_be_in_process(
    world: &mut InProcessWorld,
    step: &cucumber::gherkin::Step,
) -> Result<()> {
    check_output(world, step)
}

#[then(expr = "the output should be empty")]
async fn output_should_be_empty_full_stack(world: &mut FullStackWorld) -> Result<()> {
    check_empty_output(world)
}

#[then(expr = "the output should be empty")]
async fn output_should_be_empty_in_process(world: &mut InProcessWorld) -> Result<()> {
    check_empty_output(world)
}

// ============================================================================
// Helper functions
// ============================================================================

fn check_output<W: TestWorld>(world: &W, step: &cucumber::gherkin::Step) -> Result<()> {
    let expected = step
        .docstring
        .as_ref()
        .context("Expected docstring in step")?;

    let expected_text = expected.trim();
    let output = world.get_output();
    let actual = output.trim();
    let actual_no_ansi = strip_ansi_codes(actual);

    if actual_no_ansi != expected_text {
        anyhow::bail!(
            "\nExpected:\n{}\n\nActual:\n{}",
            expected_text,
            actual_no_ansi
        );
    }

    Ok(())
}

fn check_yak_count<W: TestWorld>(world: &mut W, expected: usize) -> Result<()> {
    world.list_yaks_with_format("plain")?;
    let output = world.get_output();
    let actual = output.trim().lines().filter(|l| !l.is_empty()).count();

    if actual != expected {
        anyhow::bail!("Expected {} yak(s), but found {}", expected, actual);
    }

    Ok(())
}

fn check_command_fails<W: TestWorld>(world: &W) -> Result<()> {
    if world.get_exit_code() == 0 {
        anyhow::bail!("Expected command to fail, but it succeeded");
    }
    Ok(())
}

fn check_error_contains<W: TestWorld>(world: &W, expected: &str) -> Result<()> {
    let error = world.get_error();
    if !error.contains(expected) {
        anyhow::bail!(
            "Expected error to contain '{}', but got: '{}'",
            expected,
            error
        );
    }
    Ok(())
}

fn check_empty_output<W: TestWorld>(world: &W) -> Result<()> {
    let output = world.get_output();
    let actual = output.trim();

    if !actual.is_empty() {
        anyhow::bail!("\nExpected empty output\n\nActual:\n{}", actual);
    }

    Ok(())
}

fn check_should_succeed<W: TestWorld>(world: &W) -> Result<()> {
    if world.get_exit_code() != 0 {
        anyhow::bail!(
            "Expected command to succeed, but it failed with exit code {}.\nstderr: {}",
            world.get_exit_code(),
            world.get_error()
        );
    }
    Ok(())
}

fn check_output_includes<W: TestWorld>(world: &W, expected: &str) -> Result<()> {
    let output = world.get_output();
    let output_no_ansi = strip_ansi_codes(&output);
    if !output_no_ansi.contains(expected) {
        anyhow::bail!(
            "Expected output to include '{}', but got:\n{}",
            expected,
            output_no_ansi
        );
    }
    Ok(())
}

fn check_line_of_output_includes<W: TestWorld>(
    world: &W,
    line_num: usize,
    expected: &str,
) -> Result<()> {
    let output = world.get_output();
    let output_no_ansi = strip_ansi_codes(&output);
    let lines: Vec<&str> = output_no_ansi.lines().collect();

    if line_num == 0 || line_num > lines.len() {
        anyhow::bail!(
            "Line {} does not exist. Output has {} line(s):\n{}",
            line_num,
            lines.len(),
            output_no_ansi
        );
    }

    let line = lines[line_num - 1];
    if !line.contains(expected) {
        anyhow::bail!(
            "Expected line {} to include '{}', but got: '{}'",
            line_num,
            expected,
            line
        );
    }
    Ok(())
}

fn check_output_not_includes<W: TestWorld>(world: &W, expected: &str) -> Result<()> {
    let output = world.get_output();
    let output_no_ansi = strip_ansi_codes(&output);
    if output_no_ansi.contains(expected) {
        anyhow::bail!(
            "Expected output to NOT include '{}', but got:\n{}",
            expected,
            output_no_ansi
        );
    }
    Ok(())
}

/// Split a string into arguments, respecting double-quoted strings.
/// `""` becomes an empty string, `"foo bar"` becomes `foo bar`.
pub fn shell_split(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut has_token = false;

    for c in s.chars() {
        match c {
            '"' => {
                has_token = true;
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if has_token {
                    result.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            _ => {
                has_token = true;
                current.push(c);
            }
        }
    }

    if has_token {
        result.push(current);
    }

    result
}
