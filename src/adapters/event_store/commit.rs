//! Commit message parsing and Event-Id trailer handling.
//!
//! This module encapsulates the logic for embedding and extracting
//! event IDs in git commit messages, used for idempotent appends
//! and cross-repo identity during sync.

use anyhow::Result;
use git2::Repository;

/// Check if any existing commit has the given Event-Id trailer.
pub(super) fn has_event_id(repo: &Repository, ref_name: &str, event_id: &str) -> Result<bool> {
    let latest = match repo.refname_to_id(ref_name) {
        Ok(oid) => repo.find_commit(oid)?,
        Err(_) => return Ok(false),
    };

    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
    revwalk.push(latest.id())?;

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let message = commit.message().unwrap_or("");
        if message_has_event_id(message, event_id) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if a commit message contains a specific Event-Id trailer.
pub(super) fn message_has_event_id(message: &str, event_id: &str) -> bool {
    let prefix = "Event-Id: ";
    for line in message.lines() {
        let trimmed = line.trim();
        if let Some(id) = trimmed.strip_prefix(prefix) {
            if id.trim() == event_id {
                return true;
            }
        }
    }
    false
}

/// Extract the Event-Id from a commit message, falling back to a
/// provided default (typically the commit SHA for legacy commits).
pub(super) fn extract_event_id(message: &str, fallback: &str) -> String {
    let prefix = "Event-Id: ";
    for line in message.lines() {
        let trimmed = line.trim();
        if let Some(id) = trimmed.strip_prefix(prefix) {
            let id = id.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
    }
    fallback.to_string()
}
