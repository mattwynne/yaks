// FullStackWorld - spawns the yx binary for full integration testing

use anyhow::{Context, Result};
use cucumber::World as CucumberWorld;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::TempDir;

use super::test_world::TestWorld;

#[derive(CucumberWorld, Debug)]
#[world(init = Self::new)]
pub struct FullStackWorld {
    repo_path: PathBuf,
    _temp_dir: TempDir,
    output: String,
    error: String,
    exit_code: i32,
    /// Override directory for scenarios that need a custom environment
    /// (e.g., git-checks tests that run without YX_SKIP_GIT_CHECKS)
    pub override_dir: Option<TempDir>,
    /// Named repositories for multi-repo scenarios (e.g., sync tests)
    pub repos: HashMap<String, TempDir>,
}

impl FullStackWorld {
    fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let repo_path = temp_dir.path().to_path_buf();

        Ok(Self {
            repo_path,
            _temp_dir: temp_dir,
            output: String::new(),
            error: String::new(),
            exit_code: 0,
            override_dir: None,
            repos: HashMap::new(),
        })
    }

    /// Get the default repository path
    pub fn default_repo_path(&self) -> &std::path::Path {
        &self.repo_path
    }

    /// Initialize git repository (needed for full-stack testing)
    pub fn init_git(&self) -> Result<()> {
        let status = Command::new("git")
            .arg("init")
            .current_dir(&self.repo_path)
            .status()
            .context("Failed to run git init")?;

        if !status.success() {
            anyhow::bail!("git init failed");
        }

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&self.repo_path)
            .status()
            .context("Failed to set git user.email")?;

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&self.repo_path)
            .status()
            .context("Failed to set git user.name")?;

        Ok(())
    }

    fn run_yx(&mut self, args: &[&str]) -> Result<()> {
        self.run_yx_unchecked(args)?;

        if !self.exit_code == 0 {
            anyhow::bail!(
                "yx command failed:\nstdout: {}\nstderr: {}",
                self.output,
                self.error
            );
        }

        Ok(())
    }

    /// Run yx with raw args, capturing output without checking exit code
    pub fn run_raw(&mut self, args: &[&str]) -> Result<()> {
        self.run_yx_unchecked(args)
    }

    fn run_yx_with_stdin(&mut self, args: &[&str], stdin_content: &str) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut child = Command::new(yx_path)
            .args(args)
            .env("YAK_PATH", &self.repo_path)
            .env("YX_SKIP_GIT_CHECKS", "1")
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn yx command")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(stdin_content.as_bytes())
                .context("Failed to write to stdin")?;
        }

        let output = child
            .wait_with_output()
            .context("Failed to wait for yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        if self.exit_code != 0 {
            anyhow::bail!(
                "yx command failed:\nstdout: {}\nstderr: {}",
                self.output,
                self.error
            );
        }

        Ok(())
    }

    /// Add a yak with piped stdin content (does not set YX_IGNORE_STDIN)
    pub fn add_yak_with_stdin(&mut self, name: &str, stdin_content: &str) -> Result<()> {
        self.run_yx_with_stdin(&["add", name], stdin_content)
    }

    /// Run yx in the override directory without YX_SKIP_GIT_CHECKS.
    /// Used for testing git environment checks (not-in-repo, no gitignore).
    pub fn run_yx_in_override_dir(&mut self, args: &[&str]) -> Result<()> {
        let dir = self
            .override_dir
            .as_ref()
            .context("No override directory set")?;
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let output = Command::new(yx_path)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("YX_IGNORE_STDIN", "1")
            .env_remove("YX_SKIP_GIT_CHECKS")
            .current_dir(dir.path())
            .output()
            .context("Failed to run yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Run bash completion by sourcing completions/yx.bash and invoking _yx_completions.
    /// The words_str is a space-separated list of words (respecting double quotes).
    /// This simulates what bash's programmable completion does.
    pub fn run_bash_completion(&mut self, words_str: &str) -> Result<()> {
        let words = super::steps::shell_split(words_str);
        let comp_cword = words.len() - 1;

        // Build COMP_WORDS array assignment for bash
        let comp_words_items: Vec<String> = words
            .iter()
            .enumerate()
            .map(|(i, w)| format!("[{}]=\"{}\"", i, w))
            .collect();
        let comp_words_str = comp_words_items.join(" ");

        // Find the project root (where completions/yx.bash lives)
        let project_dir = env!("CARGO_MANIFEST_DIR");

        let yx_path = env!("CARGO_BIN_EXE_yx");

        let script = format!(
            r#"
export YAK_PATH="{yak_path}"
export YX_SKIP_GIT_CHECKS=1
export PATH="{yx_dir}:$PATH"
source "{project_dir}/completions/yx.bash"
COMP_WORDS=({comp_words_str})
COMP_CWORD={comp_cword}
_yx_completions
printf '%s\n' "${{COMPREPLY[@]}}"
"#,
            yak_path = self.repo_path.display(),
            yx_dir = std::path::Path::new(yx_path).parent().unwrap().display(),
            project_dir = project_dir,
            comp_words_str = comp_words_str,
            comp_cword = comp_cword,
        );

        let output = Command::new("bash")
            .args(["-c", &script])
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to run bash completion script")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Get the path of a named repository
    pub fn repo_path(&self, name: &str) -> Result<PathBuf> {
        self.repos
            .get(name)
            .map(|td| td.path().to_path_buf())
            .context(format!("No repo named '{}'", name))
    }

    /// Create a bare git repository with the given name
    pub fn create_bare_repo(&mut self, name: &str) -> Result<()> {
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

        let status = Command::new("git")
            .args(["init", "--bare", "--initial-branch=main"])
            .current_dir(temp_dir.path())
            .output()
            .context("Failed to run git init --bare")?;

        if !status.status.success() {
            anyhow::bail!("git init --bare failed");
        }

        self.repos.insert(name.to_string(), temp_dir);
        Ok(())
    }

    /// Create a clone of an existing named repo.
    /// If the origin is empty, initializes with git init + remote add
    /// (matching the pattern used in ShellSpec's setup_test_repo).
    pub fn create_clone(&mut self, origin_name: &str, clone_name: &str) -> Result<()> {
        let origin_path = self.repo_path(origin_name)?;
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let clone_path = temp_dir.path();

        let hooks_env = ("GIT_CONFIG_PARAMETERS", "'core.hooksPath=/dev/null'");

        // Check if origin has any commits
        let origin_has_commits = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&origin_path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let email = format!("{}@example.com", clone_name);
        let user_name = clone_name.to_string();

        if origin_has_commits {
            // Origin has commits - use git clone
            let output = Command::new("git")
                .args([
                    "clone",
                    "--quiet",
                    &origin_path.to_string_lossy(),
                    &clone_path.to_string_lossy(),
                ])
                .env(hooks_env.0, hooks_env.1)
                .output()
                .context("Failed to git clone")?;
            if !output.status.success() {
                anyhow::bail!(
                    "git clone failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        } else {
            // Origin is empty - init repo and add remote
            Command::new("git")
                .args(["init", "--initial-branch=main", "--quiet"])
                .env(hooks_env.0, hooks_env.1)
                .current_dir(clone_path)
                .status()
                .context("Failed to git init")?;
            Command::new("git")
                .args(["remote", "add", "origin", &origin_path.to_string_lossy()])
                .current_dir(clone_path)
                .status()
                .context("Failed to add remote")?;
        }

        // Configure git user and disable hooks
        for args in [
            vec!["config", "user.email", &email],
            vec!["config", "user.name", &user_name],
            vec!["config", "core.hooksPath", "/dev/null"],
        ] {
            Command::new("git")
                .args(&args)
                .current_dir(clone_path)
                .status()?;
        }

        if !origin_has_commits {
            // Create .gitignore, commit, and push
            std::fs::write(clone_path.join(".gitignore"), ".yaks\n")
                .context("Failed to write .gitignore")?;
            Command::new("git")
                .args(["add", ".gitignore"])
                .current_dir(clone_path)
                .status()
                .context("Failed to git add")?;
            Command::new("git")
                .args(["commit", "--quiet", "-m", "Initial commit"])
                .current_dir(clone_path)
                .status()
                .context("Failed to git commit")?;
            Command::new("git")
                .args(["push", "-u", "origin", "main", "--quiet"])
                .current_dir(clone_path)
                .output()
                .context("Failed to git push")?;
        }

        self.repos.insert(clone_name.to_string(), temp_dir);
        Ok(())
    }

    /// Create a git worktree from a named parent repo
    pub fn create_worktree(&mut self, parent_name: &str, worktree_name: &str) -> Result<()> {
        let parent_path = self.repo_path(parent_name)?;
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let worktree_path = temp_dir.path();

        // Remove the temp dir since git worktree add needs a non-existent path
        std::fs::remove_dir(worktree_path).context("Failed to remove temp dir for worktree")?;

        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                &worktree_path.to_string_lossy(),
                "-b",
                worktree_name,
                "--quiet",
            ])
            .current_dir(&parent_path)
            .output()
            .context("Failed to create git worktree")?;

        if !output.status.success() {
            anyhow::bail!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        self.repos.insert(worktree_name.to_string(), temp_dir);
        Ok(())
    }

    /// Run yx command scoped to a named repository
    pub fn run_yx_in_repo(&mut self, repo_name: &str, args: &[&str]) -> Result<()> {
        let repo_path = self.repo_path(repo_name)?;
        let yak_path = repo_path.join(".yaks");
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let output = Command::new(yx_path)
            .args(args)
            .env("YAK_PATH", &yak_path)
            .env("GIT_WORK_TREE", &repo_path)
            .env("YX_IGNORE_STDIN", "1")
            .env("YX_SKIP_GIT_CHECKS", "1")
            .current_dir(&repo_path)
            .output()
            .context("Failed to run yx command in repo")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Run a git command in a named repository
    pub fn run_git_in_repo(&mut self, repo_name: &str, args: &[&str]) -> Result<()> {
        let repo_path = self.repo_path(repo_name)?;

        let output = Command::new("git")
            .args(args)
            .current_dir(&repo_path)
            .output()
            .context("Failed to run git command in repo")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Run yx with stdin redirected from a file (simulates `yx ... < file`).
    /// Unlike run_yx_with_stdin which creates a pipe (FIFO), this provides
    /// a regular file fd on stdin.
    pub fn run_yx_with_file_stdin(&mut self, args: &[&str], content: &str) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        // Write content to a temp file
        let temp_file = self.repo_path.join(".stdin_temp");
        std::fs::write(&temp_file, content).context("Failed to write temp file")?;

        let file = std::fs::File::open(&temp_file).context("Failed to open temp file")?;

        let output = Command::new(yx_path)
            .args(args)
            .env("YAK_PATH", &self.repo_path)
            .env("YX_SKIP_GIT_CHECKS", "1")
            .current_dir(&self.repo_path)
            .stdin(Stdio::from(file))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to run yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        std::fs::remove_file(&temp_file).ok();

        if self.exit_code != 0 {
            anyhow::bail!(
                "yx command failed:\nstdout: {}\nstderr: {}",
                self.output,
                self.error
            );
        }

        Ok(())
    }

    /// Run yx with piped stdin that has no content (simulates `true | yx ...`).
    /// Captures output without checking exit code.
    pub fn run_yx_with_empty_stdin(&mut self, args: &[&str]) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut child = Command::new(yx_path)
            .args(args)
            .env("YAK_PATH", &self.repo_path)
            .env("YX_SKIP_GIT_CHECKS", "1")
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn yx command")?;

        // Drop stdin immediately to simulate empty pipe
        drop(child.stdin.take());

        let output = child
            .wait_with_output()
            .context("Failed to wait for yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    fn run_yx_unchecked(&mut self, args: &[&str]) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let output = Command::new(yx_path)
            .args(args)
            .env("YAK_PATH", &self.repo_path)
            .env("YX_IGNORE_STDIN", "1")
            .env("YX_SKIP_GIT_CHECKS", "1")
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to run yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }
}

impl TestWorld for FullStackWorld {
    fn add_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["add", name])
    }

    fn add_yak_blocking(&mut self, name: &str, parent: &str) -> Result<()> {
        self.run_yx(&["add", name, "--blocks", parent])
    }

    fn try_add_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx_unchecked(&["add", name])
    }

    fn try_add_yak_blocking(&mut self, name: &str, parent: &str) -> Result<()> {
        self.run_yx_unchecked(&["add", name, "--blocks", parent])
    }

    fn remove_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["rm", name])
    }

    fn try_remove_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx_unchecked(&["rm", name])
    }

    fn get_error(&self) -> String {
        self.error.clone()
    }

    fn done_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["done", name])
    }

    fn list_yaks(&mut self) -> Result<()> {
        self.run_yx(&["list"])
    }

    fn list_yaks_with_format(&mut self, format: &str) -> Result<()> {
        self.run_yx(&["list", "--format", format])
    }

    fn list_yaks_with_format_and_filter(&mut self, format: &str, only: &str) -> Result<()> {
        self.run_yx(&["list", "--format", format, "--only", only])
    }

    fn get_output(&self) -> String {
        self.output.clone()
    }

    fn set_context(&mut self, name: &str, content: &str) -> Result<()> {
        self.run_yx_with_stdin(&["context", name], content)
    }

    fn show_context(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["context", "--show", name])
    }

    fn try_done_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx_unchecked(&["done", name])
    }

    fn done_yak_recursive(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["done", "--recursive", name])
    }

    fn prune_yaks(&mut self) -> Result<()> {
        self.run_yx(&["prune"])
    }

    fn set_state(&mut self, name: &str, state: &str) -> Result<()> {
        self.run_yx(&["state", name, state])
    }

    fn try_set_state(&mut self, name: &str, state: &str) -> Result<()> {
        self.run_yx_unchecked(&["state", name, state])
    }

    fn start_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["start", name])
    }

    fn move_yak(&mut self, from: &str, to: &str) -> Result<()> {
        self.run_yx(&["move", from, to])
    }

    fn try_move_yak(&mut self, from: &str, to: &str) -> Result<()> {
        self.run_yx_unchecked(&["move", from, to])
    }

    fn set_field(&mut self, name: &str, field: &str, content: &str) -> Result<()> {
        self.run_yx_with_stdin(&["field", name, field], content)
    }

    fn try_set_field(&mut self, name: &str, field: &str, content: &str) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut child = Command::new(yx_path)
            .args(["field", name, field])
            .env("YAK_PATH", &self.repo_path)
            .env("YX_SKIP_GIT_CHECKS", "1")
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn yx command")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(content.as_bytes())
                .context("Failed to write to stdin")?;
        }

        let output = child
            .wait_with_output()
            .context("Failed to wait for yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    fn show_field(&mut self, name: &str, field: &str) -> Result<()> {
        self.run_yx(&["field", name, field, "--show"])
    }

    fn rename_yak(&mut self, from: &str, to: &str) -> Result<()> {
        self.run_yx(&["rename", from, to])
    }

    fn try_rename_yak(&mut self, from: &str, to: &str) -> Result<()> {
        self.run_yx_unchecked(&["rename", from, to])
    }

    fn get_exit_code(&self) -> i32 {
        self.exit_code
    }
}
