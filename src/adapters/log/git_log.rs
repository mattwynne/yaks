// Git-based log adapter - commits yak operations to refs/notes/yaks

use crate::domain::events::*;
use crate::ports::LogPort;
use anyhow::{Context, Result};
use git2::Repository;
use std::path::PathBuf;

pub struct GitLog {
    repo: Option<Repository>,
    #[allow(dead_code)]
    yaks_path: PathBuf,
    git_work_tree: Option<String>,
}

impl Clone for GitLog {
    fn clone(&self) -> Self {
        // Reopen repository from work tree path
        let repo = if let Some(ref work_tree) = self.git_work_tree {
            Repository::open(work_tree).ok()
        } else {
            None
        };

        Self {
            repo,
            yaks_path: self.yaks_path.clone(),
            git_work_tree: self.git_work_tree.clone(),
        }
    }
}

#[allow(dead_code)]
impl GitLog {
    pub fn new() -> Result<Self> {
        // Skip git operations if YX_SKIP_GIT_CHECKS is set (for mutation testing and test environments)
        let skip_git_checks = std::env::var("YX_SKIP_GIT_CHECKS").is_ok();

        if skip_git_checks {
            // Return a no-op GitLog that won't try to log anything
            return Ok(Self {
                repo: None,
                yaks_path: PathBuf::from("/dev/null"), // Dummy path that doesn't exist
                git_work_tree: None,
            });
        }

        let git_work_tree = std::env::var("GIT_WORK_TREE")
            .or_else(|_| std::env::current_dir().map(|p| p.display().to_string()))?;

        let repo = Repository::open(&git_work_tree)
            .with_context(|| format!("Failed to open git repository at {git_work_tree}"))?;

        let yak_path_str = std::env::var("YAK_PATH").unwrap_or_else(|_| ".yaks".to_string());

        // Resolve yaks_path relative to git_work_tree if it's relative
        let yaks_path = if std::path::Path::new(&yak_path_str).is_absolute() {
            PathBuf::from(yak_path_str)
        } else {
            PathBuf::from(&git_work_tree).join(yak_path_str)
        };

        Ok(Self {
            repo: Some(repo),
            yaks_path,
            git_work_tree: Some(git_work_tree),
        })
    }

    // Build a tree from .yaks directory
    #[allow(dead_code)]
    fn build_tree_from_yaks(&self) -> Result<git2::Oid> {
        let repo = self
            .repo
            .as_ref()
            .expect("GitLog repo should be Some when build_tree_from_yaks is called");
        let mut index = git2::Index::new()?;

        if self.yaks_path.exists() {
            for entry in walkdir::WalkDir::new(&self.yaks_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                let relative = path.strip_prefix(&self.yaks_path)?;
                let contents = std::fs::read(path)?;

                // Create blob from file contents
                let oid = repo.blob(&contents)?;

                // Add to index
                let index_entry = git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100644, // regular file
                    uid: 0,
                    gid: 0,
                    file_size: contents.len() as u32,
                    id: oid,
                    flags: 0,
                    flags_extended: 0,
                    path: relative.to_str().unwrap().as_bytes().to_vec(),
                };
                index.add(&index_entry)?;
            }
        }

        let tree_oid = index.write_tree_to(repo)?;
        Ok(tree_oid)
    }

    // Get the OID of refs/notes/yaks if it exists
    fn get_local_ref(&self) -> Result<Option<git2::Oid>> {
        let repo = self
            .repo
            .as_ref()
            .expect("GitLog repo should be Some when get_local_ref is called");
        match repo.refname_to_id("refs/notes/yaks") {
            Ok(oid) => Ok(Some(oid)),
            Err(_) => Ok(None),
        }
    }

    // Read all events from refs/notes/yaks
    // TODO: Remove in Task 9 - legacy Event struct
    /*
    #[allow(dead_code)]
    pub fn read_events(&self) -> Result<Vec<crate::domain::Event>> {
        use chrono::{DateTime, Utc};

        // Return empty vec if repo is not available
        let repo = match &self.repo {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        // Return empty vec if no log exists yet
        let Some(ref_oid) = self.get_local_ref()? else {
            return Ok(Vec::new());
        };

        let mut events = Vec::new();
        let mut revwalk = repo.revwalk()?;
        revwalk.push(ref_oid)?;

        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            // Parse commit message as command
            let message = commit.message().unwrap_or("").trim();
            if message.is_empty() {
                continue;
            }

            // Split command into operation and args
            let parts: Vec<String> = message.split_whitespace().map(String::from).collect();
            if parts.is_empty() {
                continue;
            }

            let operation = parts[0].clone();
            let args = parts[1..].to_vec();

            // Extract timestamp
            let time = commit.time();
            let timestamp = DateTime::from_timestamp(time.seconds(), 0).unwrap_or_else(Utc::now);

            // Extract author
            let author = commit.author();
            let author_str = format!(
                "{} <{}>",
                author.name().unwrap_or("unknown"),
                author.email().unwrap_or("unknown")
            );

            events.push(crate::domain::Event::new(
                operation, args, None, // stdin not currently logged
                timestamp, author_str,
            ));
        }

        // Reverse to get chronological order (oldest first)
        events.reverse();

        Ok(events)
    }
    */
}

impl LogPort for GitLog {
    fn log_command(&self, command: &str) -> Result<()> {
        // Skip if repo is not available (YX_SKIP_GIT_CHECKS is set)
        let repo = match &self.repo {
            Some(r) => r,
            None => return Ok(()),
        };

        // Skip if yaks path doesn't exist
        if !self.yaks_path.exists() {
            return Ok(());
        }

        let tree_oid = self.build_tree_from_yaks()?;
        let tree = repo.find_tree(tree_oid)?;

        // Get parent commit if refs/notes/yaks exists
        let parent = self
            .get_local_ref()?
            .and_then(|oid| repo.find_commit(oid).ok());

        let parents: Vec<_> = parent.iter().collect();

        // Create commit
        let sig = repo.signature()?;
        repo.commit(
            Some("refs/notes/yaks"),
            &sig,
            &sig,
            command,
            &tree,
            &parents,
        )?;

        Ok(())
    }
}

use crate::domain::YakEvent;
use crate::ports::EventListener;

impl EventListener for GitLog {
    fn on_event(&mut self, event: &YakEvent) -> Result<()> {
        // Convert YakEvent to command string
        let command = match event {
            YakEvent::Added(AddedEvent { name }) => format!("add {}", name),
            YakEvent::Removed(RemovedEvent { name }) => format!("rm {}", name),
            YakEvent::Moved(MovedEvent { old_name, new_name }) => {
                format!("move {} {}", old_name, new_name)
            }
            YakEvent::ContextUpdated(ContextUpdatedEvent { name, .. }) => {
                format!("context {}", name)
            }
            YakEvent::StateUpdated(StateUpdatedEvent { name, state }) => {
                if state == "done" {
                    format!("done {}", name)
                } else if state == "todo" {
                    // TODO: This logs all todo state changes as "done --undo"
                    // which is incorrect if setting to todo for other reasons
                    // Consider adding a DoneUndone event for proper semantics
                    format!("done --undo {}", name)
                } else {
                    format!("state {} {}", name, state)
                }
            }
            YakEvent::FieldUpdated(FieldUpdatedEvent {
                name, field_name, ..
            }) => {
                format!("field {} {}", name, field_name)
            }
        };

        // Log the command using existing log_command implementation
        self.log_command(&command)
    }
}
