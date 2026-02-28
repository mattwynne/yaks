//! Sync protocol: fetch, merge, and push yak events with a remote.
//!
//! This module implements the CRDT-style sync that exchanges events
//! between local and remote refs/notes/yaks refs.

use anyhow::Result;
use std::path::Path;

use crate::domain::ports::{DisplayPort, EventStore};
use crate::domain::YakEvent;

use super::git::GitEventStore;

/// Fetch refs/notes/yaks from origin into a temporary peer ref.
/// Returns an error if sync is not configured (no origin remote).
fn fetch_peer_ref(repo_path: &Path) -> Result<()> {
    let fetch_output = std::process::Command::new("git")
        .args(["fetch", "origin", "+refs/notes/yaks:refs/notes/yaks-peer"])
        .current_dir(repo_path)
        .output();

    let has_origin = match fetch_output {
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

    if !has_origin {
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

    // 1. Fetch refs/notes/yaks from origin into a temporary peer ref
    fetch_peer_ref(&repo_path)?;

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

    output.info(&format!(
        "Pulled {} events, pushed {} events",
        merge.pulled, merge.pushed
    ));

    if let Some(ce) = received_compaction {
        output.info(&format!(
            "Received compaction from {}",
            ce.metadata().author.name
        ));
    }

    // 3. Push refs/notes/yaks back to origin
    if store.repo().refname_to_id(store.ref_name()).is_ok() {
        let push_output = std::process::Command::new("git")
            .args(["push", "origin", "+refs/notes/yaks:refs/notes/yaks"])
            .current_dir(&repo_path)
            .output()?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            anyhow::bail!("Failed to push to origin: {}", stderr.trim());
        }
    }

    // 4. Clean up the temporary peer ref
    let _ = store
        .repo()
        .find_reference("refs/notes/yaks-peer")
        .and_then(|mut r| r.delete());

    Ok(())
}
