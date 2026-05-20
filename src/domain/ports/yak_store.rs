// Yak store port traits - read/write abstractions for yak persistence

use crate::domain::slug::{Name, YakId};
use crate::domain::{Yak, YakBlockerSnapshot};
use anyhow::Result;

pub trait ReadYakStore {
    fn get_yak(&self, id: &YakId) -> Result<Yak>;
    fn list_yaks(&self) -> Result<Vec<Yak>>;
    fn fuzzy_find_yak_id(&self, query: &str) -> Result<YakId>;
    fn read_field(&self, id: &YakId, field_name: &str) -> Result<String>;

    /// Read-side projection of explicit yak-to-yak blocker facts.
    fn list_blockers(&self) -> Result<Vec<YakBlockerSnapshot>>;
}

pub trait WriteYakStore {
    /// Create a new yak. The `id` is the storage-safe identifier.
    /// If `parent_id` is Some, the yak is nested under the parent's directory.
    fn create_yak(&self, name: &Name, id: &YakId, parent_id: Option<&YakId>) -> Result<()>;

    /// Delete a yak
    fn delete_yak(&self, id: &YakId) -> Result<()>;

    /// Rename a yak
    fn rename_yak(&self, id: &YakId, new_name: &Name) -> Result<()>;

    /// Move a yak to a new parent (or to root if parent_id is None)
    fn reparent_yak(&self, id: &YakId, new_parent_id: Option<&YakId>) -> Result<()>;

    /// Write a field for a yak
    fn write_field(&self, id: &YakId, field_name: &str, content: &str) -> Result<()>;

    /// Add or replace an explicit yak-to-yak blocker in the read model.
    fn write_blocker(
        &self,
        _target: &YakId,
        _blocker: &YakId,
        _reason: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }

    /// Remove an explicit yak-to-yak blocker from the read model.
    fn remove_blocker(&self, _target: &YakId, _blocker: &YakId) -> Result<()> {
        Ok(())
    }

    /// Remove all yaks, preparing for a full replay of events.
    fn clear_all(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::slug::{Name, YakId};
    use std::collections::HashMap;

    struct InMemoryStore {
        yaks: HashMap<String, Yak>,
    }

    impl ReadYakStore for InMemoryStore {
        fn get_yak(&self, id: &YakId) -> Result<Yak> {
            self.yaks
                .get(id.as_str())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Yak not found"))
        }

        fn list_yaks(&self) -> Result<Vec<Yak>> {
            Ok(self.yaks.values().cloned().collect())
        }

        fn fuzzy_find_yak_id(&self, name: &str) -> Result<YakId> {
            if self.yaks.contains_key(name) {
                Ok(YakId::from(name))
            } else {
                anyhow::bail!("Yak not found")
            }
        }

        fn read_field(&self, _id: &YakId, _field_name: &str) -> Result<String> {
            anyhow::bail!("Field reading not implemented in test store")
        }

        fn list_blockers(&self) -> Result<Vec<YakBlockerSnapshot>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn test_store_get_yak() {
        use crate::domain::event_metadata::{Author, Timestamp};
        let mut yaks = HashMap::new();
        yaks.insert(
            "test".to_string(),
            Yak {
                id: YakId::from("test"),
                name: Name::from("test"),
                parent_id: None,
                state: crate::domain::YakState::Todo,
                context: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: Author::unknown(),
                created_at: Timestamp::zero(),
            },
        );

        let store = InMemoryStore { yaks };
        let yak = store.get_yak(&YakId::from("test")).unwrap();

        assert_eq!(yak.name, Name::from("test"));
    }
}
