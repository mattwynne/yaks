//! Sync protocol: fetch, merge, and push yak events with a remote.
//!
//! This module implements the CRDT-style sync that exchanges events
//! between local and remote refs/notes/yaks refs.

use anyhow::Result;
use std::path::Path;

use crate::adapters::views::Message;
use crate::domain::ports::{DisplayPort, EventStore};
use crate::domain::YakEvent;

use super::git::GitEventStore;

/// Resolve the sync remote (remote name or URL) to use for fetch/push.
/// Reads git config yaks.remote if set, otherwise falls back to "origin".
/// Returns an error if neither is configured.
fn resolve_sync_remote(repo_path: &Path) -> Result<String> {
    // Try to read git config yaks.remote
    let config_output = std::process::Command::new("git")
        .args(["config", "--get", "yaks.remote"])
        .current_dir(repo_path)
        .output()?;

    if config_output.status.success() {
        let remote = String::from_utf8_lossy(&config_output.stdout)
            .trim()
            .to_string();
        if !remote.is_empty() {
            return Ok(remote);
        }
    }

    // Fall back to "origin"
    Ok("origin".to_string())
}

/// Fetch refs/notes/yaks from the sync remote into a temporary peer ref.
/// Returns an error if sync is not configured (no remote).
fn fetch_peer_ref(repo_path: &Path) -> Result<()> {
    let remote = resolve_sync_remote(repo_path)?;

    let fetch_output = std::process::Command::new("git")
        .args(["fetch", &remote, "+refs/notes/yaks:refs/notes/yaks-peer"])
        .current_dir(repo_path)
        .output();

    let has_remote = match fetch_output {
        Ok(out) => {
            if out.status.success() {
                true
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                stderr.contains("couldn't find remote ref")
            }
        }
        Err(_) => false,
    };

    if !has_remote {
        anyhow::bail!("Sync not configured");
    }
    Ok(())
}

/// Execute the sync protocol: fetch, merge, replay, push.
pub(super) fn sync_with_remote(
    store: &mut GitEventStore,
    _bus: &mut crate::infrastructure::event_bus::EventBus,
    output: &dyn DisplayPort,
) -> Result<()> {
    let repo_path = store
        .repo()
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Cannot sync: bare repository"))?
        .to_path_buf();

    // 1. Fetch refs/notes/yaks from the sync remote into a temporary peer ref
    let remote = resolve_sync_remote(&repo_path)?;
    let _spinner = output.start_progress(&format!("Fetching from {}...", remote));
    fetch_peer_ref(&repo_path)?;
    drop(_spinner);

    // 2. Check peer schema version and migrate if needed
    let peer_location = super::migration::EventStoreLocation {
        repo: store.repo(),
        ref_name: "refs/notes/yaks-peer",
    };
    if let Some(peer_version) = super::migration::read_schema_version(&peer_location)? {
        if peer_version > super::migration::CURRENT_SCHEMA_VERSION {
            // Clean up peer ref before bailing
            let _ = store
                .repo()
                .find_reference("refs/notes/yaks-peer")
                .and_then(|mut r| r.delete());
            anyhow::bail!(
                "Remote yaks use schema version {} but this version of yx only supports {}. \
                 Please update yx.",
                peer_version,
                super::migration::CURRENT_SCHEMA_VERSION
            );
        }
    }

    // Migrate the peer ref to the current schema version
    super::migration::Migrator::for_current_version().ensure_schema(&peer_location)?;

    // 3. Get local and peer events
    let _spinner = output.start_progress("Merging events...");
    let local_events = EventStore::get_all_events(store)?;
    let peer = GitEventStore::with_ref_name(&repo_path, "refs/notes/yaks-peer")?;
    let peer_events = EventStore::get_all_events(&peer)?;

    let merge = super::merge_event_streams(&local_events, &peer_events);

    if merge.pulled > 0 {
        // Delete the local ref and replay all events in sorted order
        if let Ok(mut r) = store.repo().find_reference(store.ref_name()) {
            r.delete()?;
        }

        for event in &merge.events {
            store.append(event)?;
        }
    }

    // Check if we received a compaction from the peer
    let local_ids: std::collections::HashSet<String> = local_events
        .iter()
        .filter_map(|e| e.metadata().event_id.clone())
        .collect();
    let received_compaction = peer_events.iter().find(|e| {
        matches!(e, YakEvent::Compacted(_, _))
            && e.metadata()
                .event_id
                .as_ref()
                .is_some_and(|id| !local_ids.contains(id))
    });

    drop(_spinner);

    output.message(&Message::Info(format!(
        "Pulled {} events, pushed {} events",
        merge.pulled, merge.pushed
    )));

    if let Some(ce) = received_compaction {
        output.message(&Message::Info(format!(
            "Received compaction from {}",
            ce.metadata().author.name
        )));
    }

    // 3. Push refs/notes/yaks back to the sync remote
    let remote = resolve_sync_remote(&repo_path)?;
    let _spinner = output.start_progress(&format!("Pushing to {}...", remote));
    if store.repo().refname_to_id(store.ref_name()).is_ok() {
        let push_output = std::process::Command::new("git")
            .args(["push", &remote, "+refs/notes/yaks:refs/notes/yaks"])
            .current_dir(&repo_path)
            .output()?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            anyhow::bail!("Failed to push to {}: {}", remote, stderr.trim());
        }
    }

    drop(_spinner);

    // 4. Clean up the temporary peer ref
    let _ = store
        .repo()
        .find_reference("refs/notes/yaks-peer")
        .and_then(|mut r| r.delete());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_sync_remote_falls_back_to_origin_when_config_is_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a git repository
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Set yaks.remote to empty string
        std::process::Command::new("git")
            .args(["config", "yaks.remote", ""])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let result = resolve_sync_remote(repo_path).unwrap();
        assert_eq!(
            result, "origin",
            "Should fall back to 'origin' when yaks.remote is empty"
        );
    }

    #[test]
    fn resolve_sync_remote_uses_config_when_set() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a git repository
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Set yaks.remote to a custom value
        std::process::Command::new("git")
            .args(["config", "yaks.remote", "custom-remote"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let result = resolve_sync_remote(repo_path).unwrap();
        assert_eq!(
            result, "custom-remote",
            "Should use yaks.remote config when set"
        );
    }

    #[test]
    fn resolve_sync_remote_falls_back_to_origin_when_not_configured() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize a git repository
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Don't set yaks.remote

        let result = resolve_sync_remote(repo_path).unwrap();
        assert_eq!(
            result, "origin",
            "Should fall back to 'origin' when yaks.remote is not configured"
        );
    }
}
