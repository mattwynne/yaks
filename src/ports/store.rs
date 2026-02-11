use crate::domain::Yak;
use anyhow::Result;

pub trait Store {
    fn get_yak(&self, name: &str) -> Result<Yak>;
    fn list_yaks(&self) -> Result<Vec<Yak>>;
    fn yak_exists(&self, name: &str) -> bool;
    fn find_yak(&self, name: &str) -> Result<String>;
    fn read_field(&self, yak_name: &str, field_name: &str) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct InMemoryStore {
        yaks: HashMap<String, Yak>,
    }

    impl Store for InMemoryStore {
        fn get_yak(&self, name: &str) -> Result<Yak> {
            self.yaks
                .get(name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Yak not found"))
        }

        fn list_yaks(&self) -> Result<Vec<Yak>> {
            Ok(self.yaks.values().cloned().collect())
        }

        fn yak_exists(&self, name: &str) -> bool {
            self.yaks.contains_key(name)
        }

        fn find_yak(&self, name: &str) -> Result<String> {
            if self.yaks.contains_key(name) {
                Ok(name.to_string())
            } else {
                anyhow::bail!("Yak not found")
            }
        }

        fn read_field(&self, _yak_name: &str, _field_name: &str) -> Result<String> {
            anyhow::bail!("Field reading not implemented in test store")
        }
    }

    #[test]
    fn test_store_get_yak() {
        let mut yaks = HashMap::new();
        yaks.insert(
            "test".to_string(),
            Yak {
                name: "test".to_string(),
                state: "todo".to_string(),
                context: None,
                pending_events: vec![],
            },
        );

        let store = InMemoryStore { yaks };
        let yak = store.get_yak("test").unwrap();

        assert_eq!(yak.name, "test");
    }

    #[test]
    fn test_store_yak_exists() {
        let mut yaks = HashMap::new();
        yaks.insert(
            "test".to_string(),
            Yak {
                name: "test".to_string(),
                state: "todo".to_string(),
                context: None,
                pending_events: vec![],
            },
        );

        let store = InMemoryStore { yaks };

        assert!(store.yak_exists("test"));
        assert!(!store.yak_exists("missing"));
    }
}
