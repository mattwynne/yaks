// In-memory storage adapter - for testing only

use crate::domain::ports::{ReadYakStore, WriteYakStore};
use crate::domain::{Yak, CONTEXT_FIELD, STATE_FIELD};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct InMemoryStorage {
    // HashMap: yak_name -> HashMap of field_name -> field_content
    yaks: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            yaks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteYakStore for InMemoryStorage {
    fn create_yak(&self, name: &str, _id: &str, _parent_id: Option<&str>) -> Result<()> {
        let mut yaks = self.yaks.write().unwrap();

        if yaks.contains_key(name) {
            anyhow::bail!("Yak '{}' already exists", name);
        }

        let mut fields = HashMap::new();
        // Create empty context.md by default (matching DirectoryStorage behavior)
        fields.insert(CONTEXT_FIELD.to_string(), String::new());
        yaks.insert(name.to_string(), fields);

        Ok(())
    }

    fn delete_yak(&self, name: &str) -> Result<()> {
        let mut yaks = self.yaks.write().unwrap();
        yaks.remove(name);
        Ok(())
    }

    fn rename_yak(&self, from: &str, to: &str) -> Result<()> {
        let mut yaks = self.yaks.write().unwrap();

        if !yaks.contains_key(from) {
            anyhow::bail!("yak '{}' not found", from);
        }

        if yaks.contains_key(to) {
            anyhow::bail!("Yak '{}' already exists", to);
        }

        if let Some(fields) = yaks.remove(from) {
            yaks.insert(to.to_string(), fields);
        }

        Ok(())
    }

    fn write_field(&self, yak_name: &str, field_name: &str, content: &str) -> Result<()> {
        let mut yaks = self.yaks.write().unwrap();

        let fields = yaks
            .get_mut(yak_name)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", yak_name))?;

        fields.insert(field_name.to_string(), content.to_string());

        Ok(())
    }
}

impl ReadYakStore for InMemoryStorage {
    fn get_yak(&self, name: &str) -> Result<Yak> {
        let yaks = self.yaks.read().unwrap();

        let fields = yaks
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", name))?;

        // Read context field
        let context =
            fields.get(CONTEXT_FIELD).and_then(
                |c| {
                    if c.is_empty() {
                        None
                    } else {
                        Some(c.clone())
                    }
                },
            );

        // Read state field, default to "todo" if not present
        let state = fields
            .get(STATE_FIELD)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "todo".to_string());

        Ok(Yak {
            id: name.to_string(),
            name: name.to_string(),
            state,
            context,
        })
    }

    fn list_yaks(&self) -> Result<Vec<Yak>> {
        let yaks = self.yaks.read().unwrap();
        let mut result = Vec::new();

        for name in yaks.keys() {
            if let Ok(yak) = ReadYakStore::get_yak(self, name) {
                result.push(yak);
            }
        }

        // Sort by name for consistent ordering
        result.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(result)
    }

    fn yak_exists(&self, name: &str) -> bool {
        self.yaks.read().unwrap().contains_key(name)
    }

    fn find_yak(&self, name: &str) -> Result<String> {
        let yaks = self.yaks.read().unwrap();

        // First, try exact match
        if yaks.contains_key(name) {
            return Ok(name.to_string());
        }

        // If not found, try fuzzy match on the leaf node only
        let matches: Vec<String> = yaks
            .keys()
            .filter(|yak_name| {
                // Extract leaf node (last segment after /)
                let leaf = yak_name.rsplit('/').next().unwrap_or(yak_name);
                leaf.to_lowercase().contains(&name.to_lowercase())
            })
            .cloned()
            .collect();

        match matches.len() {
            0 => anyhow::bail!("yak '{}' not found", name),
            1 => Ok(matches[0].clone()),
            _ => anyhow::bail!("yak name '{}' is ambiguous", name),
        }
    }

    fn read_field(&self, yak_name: &str, field_name: &str) -> Result<String> {
        let yaks = self.yaks.read().unwrap();

        let fields = yaks
            .get(yak_name)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", yak_name))?;

        fields.get(field_name).cloned().ok_or_else(|| {
            anyhow::anyhow!("Failed to read field '{}' for '{}'", field_name, yak_name)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let storage = InMemoryStorage::new();

        // Create initial yak
        storage.create_yak("yak0", "", None).unwrap();

        let mut handles = vec![];

        // Spawn multiple threads that create yaks
        for i in 1..=5 {
            let storage_clone = storage.clone();
            let handle = thread::spawn(move || {
                storage_clone
                    .create_yak(&format!("yak{}", i), "", None)
                    .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all yaks were created
        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 6);
    }
}
