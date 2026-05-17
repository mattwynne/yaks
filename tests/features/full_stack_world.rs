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
    /// A real git repo with .yaks gitignored, used for subdirectory discovery tests
    pub git_repo: Option<TempDir>,
    /// Subdirectory path within git_repo to run commands from
    pub git_repo_subdir: Option<PathBuf>,
    /// Interactive responses to be sent to stdin (for simulating user input)
    pub interactive_responses: Vec<String>,
    /// Pending command to execute after collecting interactive responses
    pub pending_command: Option<Vec<String>>,
    /// YX_FOCUS value to set for subsequent yx commands in this scenario
    yx_focus: Option<String>,
}

impl FullStackWorld {
    fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let repo_path = temp_dir.path().to_path_buf();
        // Initialize as a real git repo with .yaks gitignored
        Self::init_git_repo_with_gitignore(&temp_dir)?;

        Ok(Self {
            repo_path,
            _temp_dir: temp_dir,
            output: String::new(),
            error: String::new(),
            exit_code: 0,
            override_dir: None,
            repos: HashMap::new(),
            git_repo: None,
            git_repo_subdir: None,
            interactive_responses: Vec::new(),
            pending_command: None,
            yx_focus: None,
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
            .env("YX_ROOT", &self.repo_path)
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

    /// Add a yak with piped stdin content
    pub fn add_yak_with_stdin(&mut self, name: &str, stdin_content: &str) -> Result<()> {
        self.run_yx_with_stdin(&["add", name], stdin_content)
    }

    /// Add a yak with --edit, using a fake $EDITOR that writes the given content
    pub fn add_yak_with_editor(&mut self, name: &str, content: &str) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        // Create a temp script that acts as EDITOR: overwrites the file
        let editor_script = self.repo_path.join(".fake-editor.sh");
        std::fs::write(
            &editor_script,
            "#!/bin/sh\nprintf '%s' \"$WRITE_TEXT\" > \"$1\"\n",
        )
        .context("Failed to write fake editor script")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&editor_script, std::fs::Permissions::from_mode(0o755))
                .context("Failed to set editor script permissions")?;
        }

        let output = Command::new(yx_path)
            .args(["add", name, "--edit"])
            .env("YX_ROOT", &self.repo_path)
            .env("EDITOR", &editor_script)
            .env("WRITE_TEXT", content)
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to run yx add --edit")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        if self.exit_code != 0 {
            anyhow::bail!(
                "yx add --edit failed:\nstdout: {}\nstderr: {}",
                self.output,
                self.error
            );
        }

        Ok(())
    }

    /// Run yx with a fake $EDITOR. The editor_script_content should be a
    /// shell script body that receives the file path as $1.
    /// Common patterns:
    ///   write:  `printf '%s' "$WRITE_TEXT" > "$1"`
    ///   append: `printf '%s' "$APPEND_TEXT" >> "$1"`
    pub fn run_yx_with_editor(
        &mut self,
        args: &[&str],
        editor_script: &str,
        editor_env: &[(&str, &str)],
    ) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let script_path = self.repo_path.join(".fake-editor.sh");
        std::fs::write(&script_path, format!("#!/bin/sh\n{}\n", editor_script))
            .context("Failed to write fake editor script")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .context("Failed to set editor script permissions")?;
        }

        let mut cmd = Command::new(yx_path);
        cmd.args(args)
            .env("YX_ROOT", &self.repo_path)
            .env("EDITOR", &script_path)
            .current_dir(&self.repo_path);

        for (key, value) in editor_env {
            cmd.env(key, value);
        }

        let output = cmd.output().context("Failed to run yx with editor")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        if self.exit_code != 0 {
            anyhow::bail!(
                "yx command with editor failed:\nstdout: {}\nstderr: {}",
                self.output,
                self.error
            );
        }

        Ok(())
    }

    /// Run yx with piped stdin AND a fake $EDITOR.
    /// Combines stdin content with editor script for testing stdin+--edit.
    pub fn run_yx_with_stdin_and_editor(
        &mut self,
        args: &[&str],
        stdin_content: &str,
        editor_script: &str,
        editor_env: &[(&str, &str)],
    ) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let script_path = self.repo_path.join(".fake-editor.sh");
        std::fs::write(&script_path, format!("#!/bin/sh\n{}\n", editor_script))
            .context("Failed to write fake editor script")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
                .context("Failed to set editor script permissions")?;
        }

        let mut cmd = Command::new(yx_path);
        cmd.args(args)
            .env("YX_ROOT", &self.repo_path)
            .env("EDITOR", &script_path)
            .current_dir(&self.repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in editor_env {
            cmd.env(key, value);
        }

        let mut child = cmd
            .spawn()
            .context("Failed to spawn yx with stdin+editor")?;

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
                "yx command with stdin+editor failed:\nstdout: {}\nstderr: {}",
                self.output,
                self.error
            );
        }

        Ok(())
    }

    /// Run yx in the override directory without YX_SKIP_GIT_CHECKS.
    /// Used for testing git environment checks (not-in-repo, no gitignore).
    pub fn run_yx_in_override_dir(&mut self, args: &[&str]) -> Result<()> {
        let dir = self
            .override_dir
            .as_ref()
            .context("No override directory set")?;
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut cmd = Command::new(yx_path);
        cmd.args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("YX_SKIP_GIT_CHECKS")
            .env_remove("YX_ROOT")
            .current_dir(dir.path());

        let output = cmd.output().context("Failed to run yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Run yx in the override directory WITH YX_SKIP_GIT_CHECKS set.
    /// Used for testing that YX_SKIP_GIT_CHECKS bypasses git requirements.
    /// yx will use .yaks in the current directory when git checks are skipped.
    pub fn run_yx_in_override_dir_skip_git_checks(&mut self, args: &[&str]) -> Result<()> {
        let dir = self
            .override_dir
            .as_ref()
            .context("No override directory set")?;
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut cmd = Command::new(yx_path);
        cmd.args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("YX_SKIP_GIT_CHECKS", "1")
            .env_remove("YX_ROOT")
            .current_dir(dir.path());

        let output = cmd.output().context("Failed to run yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Run yx in the override directory with interactive input simulation.
    /// Uses YX_FORCE_INTERACTIVE=1 to treat stdin as interactive even when piped.
    pub fn run_yx_in_override_dir_interactive(
        &mut self,
        args: &[&str],
        _extra: &str,
    ) -> Result<()> {
        let dir = self
            .override_dir
            .as_ref()
            .context("No override directory set")?;
        let yx_path = env!("CARGO_BIN_EXE_yx");

        // Collect all accumulated responses
        let stdin_input = self.interactive_responses.join("");
        self.interactive_responses.clear(); // Clear for next test

        let mut cmd = Command::new(yx_path);
        cmd.args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("YX_FORCE_INTERACTIVE", "1") // Force interactive mode
            .env_remove("YX_SKIP_GIT_CHECKS")
            .current_dir(dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn yx command")?;

        // Write all responses to stdin in a separate thread to avoid deadlock
        // Keep stdin open long enough for the child to start reading
        let stdin_thread = if let Some(mut stdin) = child.stdin.take() {
            Some(std::thread::spawn(move || -> Result<()> {
                use std::io::Write;
                // Give the child process time to start
                std::thread::sleep(std::time::Duration::from_millis(50));
                stdin.write_all(stdin_input.as_bytes())?;
                stdin.flush()?;
                // Keep stdin open a bit longer to ensure child reads it
                std::thread::sleep(std::time::Duration::from_millis(100));
                // stdin is dropped here, closing the pipe
                Ok(())
            }))
        } else {
            None
        };

        let output = child
            .wait_with_output()
            .context("Failed to wait for yx command")?;

        // Wait for stdin thread to complete
        if let Some(thread) = stdin_thread {
            thread
                .join()
                .expect("stdin thread panicked")
                .context("Failed to write to stdin in thread")?;
        }

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Create a real git repo with .yaks gitignored.
    /// Returns the repo root path.
    fn init_git_repo_with_gitignore(temp_dir: &TempDir) -> Result<()> {
        let repo_path = temp_dir.path();

        let status = Command::new("git")
            .args(["init", "--initial-branch=main"])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .current_dir(repo_path)
            .status()
            .context("Failed to run git init")?;
        if !status.success() {
            anyhow::bail!("git init failed");
        }

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .status()
            .context("Failed to set git user.email")?;

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .status()
            .context("Failed to set git user.name")?;

        // Write .gitignore with .yaks entry
        std::fs::write(repo_path.join(".gitignore"), ".yaks\n")
            .context("Failed to write .gitignore")?;

        Ok(())
    }

    /// Set up a git repo with .yaks gitignored and add a yak to it.
    /// Stores the repo in self.git_repo.
    pub fn setup_git_repo_with_yak(&mut self, yak_name: &str) -> Result<()> {
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        Self::init_git_repo_with_gitignore(&temp_dir)?;

        let repo_path = temp_dir.path().to_path_buf();
        let yx_path = env!("CARGO_BIN_EXE_yx");

        // Add the yak from the repo root
        let output = Command::new(yx_path)
            .args(["add", yak_name])
            .env("YX_ROOT", &repo_path)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .current_dir(&repo_path)
            .output()
            .context("Failed to run yx add")?;

        if !output.status.success() {
            anyhow::bail!(
                "yx add failed:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }

        self.git_repo = Some(temp_dir);
        Ok(())
    }

    /// Create a subdirectory within the git_repo.
    pub fn create_subdir_in_git_repo(&mut self, subdir_name: &str) -> Result<()> {
        let repo = self
            .git_repo
            .as_ref()
            .context("No git_repo set; call setup_git_repo_with_yak first")?;
        let subdir = repo.path().join(subdir_name);
        std::fs::create_dir_all(&subdir).context("Failed to create subdirectory")?;
        self.git_repo_subdir = Some(subdir);
        Ok(())
    }

    /// Run yx ls from the git_repo subdirectory (if set) or the repo root,
    /// without YX_SKIP_GIT_CHECKS. Does NOT pass YX_ROOT (lets yx discover it).
    pub fn list_yaks_from_subdir(&mut self) -> Result<()> {
        let repo = self.git_repo.as_ref().context("No git_repo set")?;
        let run_dir = self
            .git_repo_subdir
            .clone()
            .unwrap_or_else(|| repo.path().to_path_buf());
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let output = Command::new(yx_path)
            .args(["ls"])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("YX_SKIP_GIT_CHECKS")
            .env_remove("YX_ROOT")
            .current_dir(&run_dir)
            .output()
            .context("Failed to run yx ls")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Run yx with YX_ROOT pointing to a specific directory.
    /// Used for testing YX_ROOT override behavior.
    pub fn run_yx_with_yx_root(
        &mut self,
        args: &[&str],
        root_path: &std::path::Path,
    ) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");
        let output = Command::new(yx_path)
            .args(args)
            .env("YX_ROOT", root_path)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("YX_SKIP_GIT_CHECKS")
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
export YX_ROOT="{yx_root}"
export PATH="{yx_dir}:$PATH"
source "{project_dir}/completions/yx.bash"
COMP_WORDS=({comp_words_str})
COMP_CWORD={comp_cword}
_yx_completions
printf '%s\n' "${{COMPREPLY[@]}}"
"#,
            yx_root = self.repo_path.display(),
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

    /// Make a repo's origin remote unreachable by pointing it at a
    /// non-existent path. This tests that non-sync commands don't
    /// contact the remote.
    pub fn make_origin_unreachable(&self, repo_name: &str) -> Result<()> {
        let repo_path = self.repo_path(repo_name)?;
        Command::new("git")
            .args(["remote", "set-url", "origin", "file:///nonexistent/repo"])
            .current_dir(&repo_path)
            .status()
            .context("Failed to set remote URL")?;
        Ok(())
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

        // Check if origin has any commits (cat-file -t HEAD fails on empty repos,
        // unlike rev-parse HEAD which misleadingly succeeds)
        let origin_has_commits = Command::new("git")
            .args(["cat-file", "-t", "HEAD"])
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

    /// Create a clone of an existing named repo using a file:// URL.
    /// This exercises the same fetch/push code path as SSH/HTTPS URLs.
    pub fn create_clone_with_file_url(
        &mut self,
        origin_name: &str,
        clone_name: &str,
    ) -> Result<()> {
        let origin_path = self.repo_path(origin_name)?;
        let file_url = format!("file://{}", origin_path.display());
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let clone_path = temp_dir.path();

        let hooks_env = ("GIT_CONFIG_PARAMETERS", "'core.hooksPath=/dev/null'");

        // Check if origin has any commits (cat-file -t HEAD fails on empty repos,
        // unlike rev-parse HEAD which misleadingly succeeds)
        let origin_has_commits = Command::new("git")
            .args(["cat-file", "-t", "HEAD"])
            .current_dir(&origin_path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let email = format!("{}@example.com", clone_name);
        let user_name = clone_name.to_string();

        if origin_has_commits {
            let output = Command::new("git")
                .args(["clone", "--quiet", &file_url, &clone_path.to_string_lossy()])
                .env(hooks_env.0, hooks_env.1)
                .output()
                .context("Failed to git clone with file URL")?;
            if !output.status.success() {
                anyhow::bail!(
                    "git clone failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        } else {
            Command::new("git")
                .args(["init", "--initial-branch=main", "--quiet"])
                .env(hooks_env.0, hooks_env.1)
                .current_dir(clone_path)
                .status()
                .context("Failed to git init")?;
            Command::new("git")
                .args(["remote", "add", "origin", &file_url])
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

    /// Run yx command scoped to a named repository with stdin content
    pub fn run_yx_in_repo_with_stdin(
        &mut self,
        repo_name: &str,
        args: &[&str],
        stdin_content: &str,
    ) -> Result<()> {
        let repo_path = self.repo_path(repo_name)?;
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut child = Command::new(yx_path)
            .args(args)
            .env("YX_ROOT", &repo_path)
            .current_dir(&repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn yx command in repo")?;

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

        Ok(())
    }

    /// Run yx command scoped to a named repository
    pub fn run_yx_in_repo(&mut self, repo_name: &str, args: &[&str]) -> Result<()> {
        let repo_path = self.repo_path(repo_name)?;
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let output = Command::new(yx_path)
            .args(args)
            .env("YX_ROOT", &repo_path)
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
            .env("YX_ROOT", &self.repo_path)
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

    /// Run yx with piped stdin content, capturing output without checking exit code.
    /// Used for testing error cases where stdin + flags conflict.
    pub fn run_yx_with_stdin_unchecked(
        &mut self,
        args: &[&str],
        stdin_content: &str,
    ) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut child = Command::new(yx_path)
            .args(args)
            .env("YX_ROOT", &self.repo_path)
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

        Ok(())
    }

    /// Run yx with piped stdin that has no content (simulates `true | yx ...`).
    /// Captures output without checking exit code.
    pub fn run_yx_with_empty_stdin(&mut self, args: &[&str]) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut child = Command::new(yx_path)
            .args(args)
            .env("YX_ROOT", &self.repo_path)
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

        let mut cmd = Command::new(yx_path);
        cmd.args(args)
            .env("YX_ROOT", &self.repo_path)
            .current_dir(&self.repo_path);

        if let Some(focus) = &self.yx_focus {
            cmd.env("YX_FOCUS", focus);
        }

        let output = cmd.output().context("Failed to run yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Run yx outside a Claude Code session (CLAUDECODE explicitly removed).
    pub fn run_yx_outside_claude_session(&mut self, args: &[&str]) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut cmd = Command::new(yx_path);
        cmd.args(args)
            .env("YX_ROOT", &self.repo_path)
            .env_remove("CLAUDECODE")
            .current_dir(&self.repo_path);

        let output = cmd.output().context("Failed to run yx command")?;
        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(())
    }

    /// Run yx with extra environment variables, capturing output without
    /// checking exit code.
    pub fn run_yx_with_env(&mut self, args: &[&str], envs: &[(&str, &str)]) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut cmd = Command::new(yx_path);
        cmd.args(args)
            .env("YX_ROOT", &self.repo_path)
            .current_dir(&self.repo_path);

        for (key, value) in envs {
            cmd.env(key, value);
        }

        let output = cmd.output().context("Failed to run yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }
}

impl TestWorld for FullStackWorld {
    fn set_yx_focus(&mut self, focus: &str) -> Result<()> {
        self.yx_focus = Some(focus.to_string());
        Ok(())
    }

    fn add_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["add", name])
    }

    fn add_yak_under(&mut self, name: &str, parent: &str) -> Result<()> {
        self.run_yx(&["add", name, "--under", parent])
    }

    fn try_add_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx_unchecked(&["add", name])
    }

    fn try_add_yak_under(&mut self, name: &str, parent: &str) -> Result<()> {
        self.run_yx_unchecked(&["add", name, "--under", parent])
    }

    fn remove_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["rm", name])
    }

    fn remove_yak_recursive(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["rm", name, "--recursive"])
    }

    fn try_remove_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx_unchecked(&["rm", name])
    }

    fn add_blocker(&mut self, target: &str, blocker: &str, reason: Option<&str>) -> Result<()> {
        match reason {
            Some(reason) => self.run_yx(&[
                "blocker", "add", target, "--by", blocker, "--reason", reason,
            ]),
            None => self.run_yx(&["blocker", "add", target, "--by", blocker]),
        }
    }

    fn try_add_blocker(&mut self, target: &str, blocker: &str, reason: Option<&str>) -> Result<()> {
        match reason {
            Some(reason) => self.run_yx_unchecked(&[
                "blocker", "add", target, "--by", blocker, "--reason", reason,
            ]),
            None => self.run_yx_unchecked(&["blocker", "add", target, "--by", blocker]),
        }
    }

    fn remove_blocker(&mut self, target: &str, blocker: &str) -> Result<()> {
        self.run_yx(&["blocker", "rm", target, "--by", blocker])
    }

    fn add_manual_blocker(&mut self, target: &str, reason: &str) -> Result<()> {
        self.run_yx(&["blocker", "add", target, "--reason", reason])
    }

    fn try_add_manual_blocker(&mut self, target: &str, reason: Option<&str>) -> Result<()> {
        match reason {
            Some(reason) => self.run_yx_unchecked(&["blocker", "add", target, "--reason", reason]),
            None => self.run_yx_unchecked(&["blocker", "add", target]),
        }
    }

    fn remove_manual_blocker(&mut self, target: &str) -> Result<()> {
        self.run_yx(&["blocker", "rm", target, "--manual"])
    }

    fn show_log(&mut self) -> Result<()> {
        self.run_yx(&["log"])
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

    fn list_yaks_ready(&mut self) -> Result<()> {
        self.run_yx(&["list", "--ready"])
    }

    fn list_yaks_with_format(&mut self, format: &str) -> Result<()> {
        self.run_yx(&["list", "--format", format])
    }

    fn list_yaks_with_format_and_filter(&mut self, format: &str, only: &str) -> Result<()> {
        self.run_yx(&["list", "--format", format, "--only", only])
    }

    fn list_yaks_json(&mut self) -> Result<()> {
        self.run_yx(&["list", "--format", "json"])
    }

    fn list_yaks_with_format_and_tag(&mut self, format: &str, tag: &str) -> Result<()> {
        self.run_yx(&["list", "--format", format, "--tag", tag])
    }

    fn list_yaks_with_format_tag_and_filter(
        &mut self,
        format: &str,
        tag: &str,
        only: &str,
    ) -> Result<()> {
        self.run_yx(&["list", "--format", format, "--tag", tag, "--only", only])
    }

    fn try_list_yaks_with_format(&mut self, format: &str) -> Result<()> {
        self.run_yx_unchecked(&["list", "--format", format])
    }

    fn try_list_yaks_with_filter(&mut self, only: &str) -> Result<()> {
        self.run_yx_unchecked(&["list", "--only", only])
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

    fn prune_yaks_excluding_tag(&mut self, tag: &str) -> Result<()> {
        self.run_yx(&["prune", "--exclude-tag", tag])
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

    fn start_yak_recursive(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["start", "--recursive", name])
    }

    fn try_start_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx_unchecked(&["start", name])
    }

    fn move_yak_under(&mut self, name: &str, parent: &str) -> Result<()> {
        self.run_yx(&["move", name, "--under", parent])
    }

    fn move_yak_to_root(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["move", name, "--to-root"])
    }

    fn try_move_yak_under_and_to_root(&mut self, name: &str, parent: &str) -> Result<()> {
        self.run_yx_unchecked(&["move", name, "--under", parent, "--to-root"])
    }

    fn try_move_yak_no_flags(&mut self, name: &str) -> Result<()> {
        self.run_yx_unchecked(&["move", name])
    }

    fn try_move_yak_under(&mut self, name: &str, parent: &str) -> Result<()> {
        self.run_yx_unchecked(&["move", name, "--under", parent])
    }

    fn set_field(&mut self, name: &str, field: &str, content: &str) -> Result<()> {
        self.run_yx_with_stdin(&["field", name, field], content)
    }

    fn try_set_field(&mut self, name: &str, field: &str, content: &str) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let mut child = Command::new(yx_path)
            .args(["field", name, field])
            .env("YX_ROOT", &self.repo_path)
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

    fn add_yak_with_state(&mut self, name: &str, state: &str) -> Result<()> {
        self.run_yx(&["add", name, "--state", state])
    }

    fn add_yak_with_context(&mut self, name: &str, context: &str) -> Result<()> {
        self.run_yx(&["add", name, "--context", context])
    }

    fn add_yak_with_id(&mut self, name: &str, id: &str) -> Result<()> {
        self.run_yx(&["add", name, "--id", id])
    }

    fn add_yak_with_id_under(&mut self, name: &str, id: &str, parent: &str) -> Result<()> {
        self.run_yx(&["add", name, "--id", id, "--under", parent])
    }

    fn add_yak_with_field(&mut self, name: &str, key: &str, value: &str) -> Result<()> {
        let field_arg = format!("{}={}", key, value);
        self.run_yx(&["add", name, "--field", &field_arg])
    }

    fn add_tags(&mut self, name: &str, tags: Vec<String>) -> Result<()> {
        let mut args = vec!["tag", "add", name];
        let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
        args.extend(tag_refs);
        self.run_yx(&args)
    }

    fn remove_tags(&mut self, name: &str, tags: Vec<String>) -> Result<()> {
        let mut args = vec!["tag", "rm", name];
        let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
        args.extend(tag_refs);
        self.run_yx(&args)
    }

    fn list_tags(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["tag", "list", name])
    }

    fn create_bare_repo(&mut self, name: &str) -> Result<()> {
        FullStackWorld::create_bare_repo(self, name)
    }

    fn create_clone(&mut self, origin: &str, clone: &str) -> Result<()> {
        FullStackWorld::create_clone(self, origin, clone)
    }

    fn get_exit_code(&self) -> i32 {
        self.exit_code
    }
}

impl FullStackWorld {
    /// Run yx with NO_COLOR=1 set, capturing raw output (may contain ANSI codes)
    pub fn run_yx_with_no_color(&mut self, args: &[&str]) -> Result<()> {
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let output = Command::new(yx_path)
            .args(args)
            .env("YX_ROOT", &self.repo_path)
            .env("NO_COLOR", "1")
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to run yx command with NO_COLOR")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();
        self.error = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(())
    }

    /// Set git config yaks.remote in a named repository
    pub fn set_sync_remote(&mut self, repo_name: &str, remote_url: &str) -> Result<()> {
        let repo_path = self.repo_path(repo_name)?;

        let output = Command::new("git")
            .args(["config", "yaks.remote", remote_url])
            .current_dir(&repo_path)
            .output()
            .context("Failed to set yaks.remote config")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to set yaks.remote: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Get the URL (path) of a named repository
    pub fn get_repo_url(&self, repo_name: &str) -> Result<String> {
        let repo_path = self.repo_path(repo_name)?;
        Ok(repo_path.to_string_lossy().to_string())
    }

    /// Check if yaks.remote is configured in a named repository
    pub fn has_sync_remote_configured(&self, repo_name: &str) -> Result<bool> {
        let repo_path = self.repo_path(repo_name)?;

        let output = Command::new("git")
            .args(["config", "--get", "yaks.remote"])
            .current_dir(&repo_path)
            .output()
            .context("Failed to check yaks.remote config")?;

        Ok(output.status.success())
    }
}
