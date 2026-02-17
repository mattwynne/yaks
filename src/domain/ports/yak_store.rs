// Yak store port traits - read/write abstractions for yak persistence

use crate::domain::slug::YakId;
use crate::domain::Yak;
use anyhow::Result;

pub trait ReadYakStore {
    fn get_yak(&self, id: &YakId) -> Result<Yak>;
    fn list_yaks(&self) -> Result<Vec<Yak>>;
    // TODO: Change yak_exists to accept &YakId instead of &str
    fn yak_exists(&self, name: &str) -> bool;
    fn fuzzy_find_yak_id(&self, query: &str) -> Result<YakId>;
    fn read_field(&self, id: &YakId, field_name: &str) -> Result<String>;
}

pub trait WriteYakStore {
    /// Create a new yak. The `id` is the storage-safe identifier.
    /// If `parent_id` is Some, the yak is nested under the parent's directory.
    fn create_yak(&self, name: &str, id: &str, parent_id: Option<&str>) -> Result<()>;

    /// Delete a yak
    fn delete_yak(&self, name: &str) -> Result<()>;

    /// Rename a yak
    fn rename_yak(&self, from: &str, to: &str) -> Result<()>;

    /// Move a yak to a new parent (or to root if parent_id is None)
    fn reparent_yak(&self, id: &str, new_parent_id: Option<&str>) -> Result<()>;

    /// Write a field for a yak
    fn write_field(&self, yak_name: &str, field_name: &str, content: &str) -> Result<()>;
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

        fn yak_exists(&self, name: &str) -> bool {
            self.yaks.contains_key(name)
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
    }

    #[test]
    fn test_store_get_yak() {
        let mut yaks = HashMap::new();
        yaks.insert(
            "test".to_string(),
            Yak {
                id: YakId::from("test"),
                name: Name::from("test"),
                state: "todo".to_string(),
                context: None,
            },
        );

        let store = InMemoryStore { yaks };
        let yak = store.get_yak(&YakId::from("test")).unwrap();

        assert_eq!(yak.name, Name::from("test"));
    }

    #[test]
    fn test_store_yak_exists() {
        let mut yaks = HashMap::new();
        yaks.insert(
            "test".to_string(),
            Yak {
                id: YakId::from("test"),
                name: Name::from("test"),
                state: "todo".to_string(),
                context: None,
            },
        );

        let store = InMemoryStore { yaks };

        assert!(store.yak_exists("test"));
        assert!(!store.yak_exists("missing"));
    }
}
