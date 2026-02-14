// Sync port trait - abstraction for git ref synchronization

use anyhow::Result;

pub trait SyncPort {
    /// Sync yaks (push + pull with merge)
    fn sync(&self) -> Result<()>;
}
