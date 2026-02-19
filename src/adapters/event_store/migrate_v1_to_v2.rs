use anyhow::Result;
use git2::Repository;

use super::migration::Migration;

/// No-op migration from v1 to v2.
/// Placeholder — will be fleshed out when yak names/IDs/paths lands.
pub struct MigrateV1ToV2;

impl Migration for MigrateV1ToV2 {
    fn source_version(&self) -> u32 {
        1
    }
    fn target_version(&self) -> u32 {
        2
    }
    fn migrate(&self, _repo: &Repository) -> Result<()> {
        // No-op for now. Future: transform events for slug-based IDs.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::event_store::migration::Migration;

    #[test]
    fn version_constants() {
        let m = MigrateV1ToV2;
        assert_eq!(m.source_version(), 1);
        assert_eq!(m.target_version(), 2);
    }
}
