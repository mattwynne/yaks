use anyhow::{Context, Result};
use cucumber::World as CucumberWorld;
use std::path::PathBuf;
use tempfile::TempDir;

use yx::adapters::{InMemoryLog, InMemoryOutput, InMemoryStorage};

#[derive(Debug)]
pub enum ExecutionMode {
    FullStack,
    InProcess,
}

#[derive(CucumberWorld)]
#[world(init = Self::new)]
pub struct World {
    pub mode: ExecutionMode,
    pub repo_path: PathBuf,
    pub _temp_dir: TempDir,
    pub output: String,
    pub exit_code: i32,
    // In-process mode adapters (only used in InProcess mode)
    pub storage: InMemoryStorage,
    pub output_adapter: InMemoryOutput,
    pub log: InMemoryLog,
}

// Manual Debug implementation to skip adapter fields
impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("mode", &self.mode)
            .field("repo_path", &self.repo_path)
            .field("output", &self.output)
            .field("exit_code", &self.exit_code)
            .finish()
    }
}

impl World {
    fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let repo_path = temp_dir.path().to_path_buf();

        // Read CUCUMBER_MODE env var to determine mode
        let mode = match std::env::var("CUCUMBER_MODE").as_deref() {
            Ok("in-process") => ExecutionMode::InProcess,
            _ => ExecutionMode::FullStack,
        };

        Ok(Self {
            mode,
            repo_path,
            _temp_dir: temp_dir,
            output: String::new(),
            exit_code: 0,
            storage: InMemoryStorage::new(),
            output_adapter: InMemoryOutput::new(),
            log: InMemoryLog::new(),
        })
    }
}

pub fn strip_ansi_codes(s: &str) -> String {
    // Simple ANSI code stripper - matches ESC[...m patterns
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}
