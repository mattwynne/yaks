// FullStackWorld - spawns the yx binary for full integration testing

use anyhow::{Context, Result};
use cucumber::World as CucumberWorld;
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
        })
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

    fn try_add_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx_unchecked(&["add", name])
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

    fn get_exit_code(&self) -> i32 {
        self.exit_code
    }
}
