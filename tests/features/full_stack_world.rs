// FullStackWorld - spawns the yx binary for full integration testing

use anyhow::{Context, Result};
use cucumber::World as CucumberWorld;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

use super::test_world::TestWorld;

#[derive(CucumberWorld, Debug)]
#[world(init = Self::new)]
pub struct FullStackWorld {
    repo_path: PathBuf,
    _temp_dir: TempDir,
    output: String,
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
        let yx_path = env!("CARGO_BIN_EXE_yx");

        let output = Command::new(yx_path)
            .args(args)
            .env("YAK_PATH", &self.repo_path)
            .env("YX_IGNORE_STDIN", "1") // Skip interactive editor
            .env("YX_SKIP_GIT_CHECKS", "1") // Skip git logging
            .current_dir(&self.repo_path)
            .output()
            .context("Failed to run yx command")?;

        self.exit_code = output.status.code().unwrap_or(-1);
        self.output = String::from_utf8_lossy(&output.stdout).to_string();

        if !output.status.success() {
            anyhow::bail!(
                "yx command failed:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }
}

impl TestWorld for FullStackWorld {
    fn add_yak(&mut self, name: &str) -> Result<()> {
        self.run_yx(&["add", name])
    }

    fn list_yaks(&mut self) -> Result<()> {
        self.run_yx(&["list"])
    }

    fn get_output(&self) -> String {
        self.output.clone()
    }

    fn get_exit_code(&self) -> i32 {
        self.exit_code
    }
}
