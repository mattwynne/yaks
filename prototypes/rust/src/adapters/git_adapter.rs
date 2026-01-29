use crate::ports::GitRepository;
use anyhow::{bail, Context, Result};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

pub struct GitAdapter {
    work_tree: PathBuf,
    yaks_path: PathBuf,
}

impl GitAdapter {
    #[must_use]
    pub fn new(work_tree: PathBuf, yaks_path: PathBuf) -> Self {
        Self {
            work_tree,
            yaks_path,
        }
    }

    fn run_git(&self, args: &[&str]) -> Result<std::process::Output> {
        Command::new("git")
            .args(["-C", self.work_tree.to_str().unwrap()])
            .args(args)
            .output()
            .context("Failed to execute git command")
    }

    fn run_git_success(&self, args: &[&str]) -> Result<bool> {
        Ok(self.run_git(args)?.status.success())
    }

    fn run_git_output(&self, args: &[&str]) -> Result<String> {
        let output = self.run_git(args)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            bail!(
                "Git command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        }
    }

    fn get_local_ref(&self) -> Result<Option<String>> {
        let output = self.run_git(&["rev-parse", "refs/notes/yaks"])?;
        if output.status.success() {
            Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
        } else {
            Ok(None)
        }
    }

    fn get_remote_ref(&self) -> Result<Option<String>> {
        let output = self.run_git(&["rev-parse", "refs/remotes/origin/yaks"])?;
        if output.status.success() {
            Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
        } else {
            Ok(None)
        }
    }

    fn yaks_path_has_content(&self) -> bool {
        self.yaks_path.exists()
            && std::fs::read_dir(&self.yaks_path)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(false)
    }

    fn extract_yaks_to_working_dir(&self) -> Result<()> {
        if self.yaks_path.exists() {
            std::fs::remove_dir_all(&self.yaks_path)?;
        }
        std::fs::create_dir_all(&self.yaks_path)?;

        if self.get_local_ref()?.is_some() {
            let archive_output = Command::new("git")
                .args(["-C", self.work_tree.to_str().unwrap()])
                .args(["archive", "refs/notes/yaks"])
                .output()?;

            if archive_output.status.success() {
                Command::new("tar")
                    .args(["-x", "-C", self.yaks_path.to_str().unwrap()])
                    .stdin(std::process::Stdio::piped())
                    .spawn()?
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(&archive_output.stdout)?;
            }
        }

        Ok(())
    }
}

impl GitRepository for GitAdapter {
    fn is_repository(&self) -> bool {
        self.run_git_success(&["rev-parse", "--git-dir"])
            .unwrap_or(false)
    }

    fn has_origin_remote(&self) -> bool {
        self.run_git_success(&["remote", "get-url", "origin"])
            .unwrap_or(false)
    }

    fn check_ignore(&self, path: &str) -> Result<bool> {
        self.run_git_success(&["check-ignore", "-q", path])
    }

    fn log_command(&self, message: &str) -> Result<()> {
        if !self.is_repository() || !self.yaks_path_has_content() {
            return Ok(());
        }

        let temp_index = std::env::temp_dir().join(format!("yx-index-{}", std::process::id()));

        std::env::set_var("GIT_INDEX_FILE", &temp_index);
        std::env::set_var("GIT_WORK_TREE", &self.yaks_path);

        let _ = self.run_git(&["read-tree", "--empty"]);
        let _ = self.run_git(&["add", "."]);
        let tree = self.run_git_output(&["write-tree"])?;

        let mut commit_args = vec!["commit-tree", &tree];
        let parent_args;
        if let Some(parent) = self.get_local_ref()? {
            parent_args = format!("-p{parent}");
            commit_args.push(&parent_args);
        }
        commit_args.extend(&["-m", message]);

        let new_commit = self.run_git_output(&commit_args)?;
        self.run_git(&["update-ref", "refs/notes/yaks", &new_commit])?;

        std::env::remove_var("GIT_INDEX_FILE");
        std::env::remove_var("GIT_WORK_TREE");
        let _ = std::fs::remove_file(temp_index);

        Ok(())
    }

    fn sync(&self) -> Result<()> {
        if !self.is_repository() {
            bail!("Error: not in a git repository");
        }
        if !self.has_origin_remote() {
            bail!("Error: no origin remote configured");
        }

        let _ = self.run_git(&[
            "fetch",
            "origin",
            "refs/notes/yaks:refs/remotes/origin/yaks",
        ]);

        let remote_ref = self.get_remote_ref()?;
        let _local_ref = self.get_local_ref()?;

        if self.yaks_path_has_content() && remote_ref.is_some() {
            self.log_command("sync")?;
        } else if !self.yaks_path_has_content() && remote_ref.is_some() {
            if let Some(remote) = remote_ref {
                self.run_git(&["update-ref", "refs/notes/yaks", &remote])?;
            }
        }

        if self.get_local_ref()?.is_some() {
            let _ = self.run_git(&["push", "origin", "refs/notes/yaks:refs/notes/yaks"]);
        }

        self.extract_yaks_to_working_dir()?;

        let _ = self.run_git(&["update-ref", "-d", "refs/remotes/origin/yaks"]);

        Ok(())
    }
}
