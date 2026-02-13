// In-memory storage adapter - for testing only

use crate::domain::events::*;
use crate::domain::{Yak, YakEvent, CONTEXT_FIELD, STATE_FIELD};
use crate::ports::{EventListener, StoragePort, Store};
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

impl StoragePort for InMemoryStorage {
    fn create_yak(&self, name: &str) -> Result<()> {
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
            name: name.to_string(),
            state,
            context,
        })
    }

    fn list_yaks(&self) -> Result<Vec<Yak>> {
        let yaks = self.yaks.read().unwrap();
        let mut result = Vec::new();

        for name in yaks.keys() {
            if let Ok(yak) = StoragePort::get_yak(self, name) {
                result.push(yak);
            }
        }

        // Sort by name for consistent ordering
        result.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(result)
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

    fn write_field(&self, yak_name: &str, field_name: &str, content: &str) -> Result<()> {
        let mut yaks = self.yaks.write().unwrap();

        let fields = yaks
            .get_mut(yak_name)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", yak_name))?;

        fields.insert(field_name.to_string(), content.to_string());

        Ok(())
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

impl EventListener for InMemoryStorage {
    fn on_event(&mut self, event: &YakEvent) -> Result<()> {
        match event {
            YakEvent::Added(AddedEvent { name }) => {
                self.create_yak(name)?;
                // Set default state
                self.write_field(name, STATE_FIELD, "todo")?;
            }

            YakEvent::Removed(RemovedEvent { name }) => {
                self.delete_yak(name)?;
            }

            YakEvent::Moved(MovedEvent { old_name, new_name }) => {
                self.rename_yak(old_name, new_name)?;
            }

            YakEvent::ContextUpdated(ContextUpdatedEvent { name, content }) => {
                self.write_field(name, CONTEXT_FIELD, content)?;
            }

            YakEvent::StateUpdated(StateUpdatedEvent { name, state }) => {
                self.write_field(name, STATE_FIELD, state)?;
            }

            YakEvent::FieldUpdated(FieldUpdatedEvent {
                name,
                field_name,
                content,
            }) => {
                self.write_field(name, field_name, content)?;
            }
        }
        Ok(())
    }
}

impl Store for InMemoryStorage {
    fn get_yak(&self, name: &str) -> Result<Yak> {
        StoragePort::get_yak(self, name)
    }

    fn list_yaks(&self) -> Result<Vec<Yak>> {
        StoragePort::list_yaks(self)
    }

    fn yak_exists(&self, name: &str) -> bool {
        self.yaks.read().unwrap().contains_key(name)
    }

    fn find_yak(&self, name: &str) -> Result<String> {
        StoragePort::find_yak(self, name)
    }

    fn read_field(&self, yak_name: &str, field_name: &str) -> Result<String> {
        StoragePort::read_field(self, yak_name, field_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_yak() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();

        // Verify yak exists
        let yak = StoragePort::get_yak(&storage, "test-yak").unwrap();
        assert_eq!(yak.name, "test-yak");
    }

    #[test]
    fn test_create_duplicate_yak() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        let result = storage.create_yak("test-yak");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_get_yak() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        let yak = StoragePort::get_yak(&storage, "test-yak").unwrap();
        assert_eq!(yak.name, "test-yak");
        assert!(!yak.is_done());
        assert_eq!(yak.state, "todo");
        assert_eq!(yak.context, None);
    }

    #[test]
    fn test_get_nonexistent_yak() {
        let storage = InMemoryStorage::new();
        let result = StoragePort::get_yak(&storage, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_list_yaks() {
        let storage = InMemoryStorage::new();
        storage.create_yak("yak1").unwrap();
        storage.create_yak("yak2").unwrap();
        let yaks = StoragePort::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 2);
        // Check sorted order
        assert_eq!(yaks[0].name, "yak1");
        assert_eq!(yaks[1].name, "yak2");
    }

    #[test]
    fn test_list_yaks_empty() {
        let storage = InMemoryStorage::new();
        let yaks = StoragePort::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 0);
    }

    #[test]
    fn test_delete_yak() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        storage.delete_yak("test-yak").unwrap();
        let result = StoragePort::get_yak(&storage, "test-yak");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent_yak() {
        let storage = InMemoryStorage::new();
        // Should not error (matches DirectoryStorage behavior)
        let result = storage.delete_yak("nonexistent");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rename_yak() {
        let storage = InMemoryStorage::new();
        storage.create_yak("old-name").unwrap();
        storage
            .write_field("old-name", CONTEXT_FIELD, "Context text")
            .unwrap();
        storage
            .write_field("old-name", STATE_FIELD, "done")
            .unwrap();

        storage.rename_yak("old-name", "new-name").unwrap();

        // Old name should not exist
        let result = StoragePort::get_yak(&storage, "old-name");
        assert!(result.is_err());

        // New name should exist with all fields preserved
        let yak = StoragePort::get_yak(&storage, "new-name").unwrap();
        assert_eq!(yak.name, "new-name");
        assert!(yak.is_done());
        assert_eq!(yak.context.unwrap(), "Context text");
    }

    #[test]
    fn test_rename_nonexistent_yak() {
        let storage = InMemoryStorage::new();
        let result = storage.rename_yak("nonexistent", "new-name");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_rename_to_existing_yak() {
        let storage = InMemoryStorage::new();
        storage.create_yak("yak1").unwrap();
        storage.create_yak("yak2").unwrap();
        let result = storage.rename_yak("yak1", "yak2");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_find_yak_exact_match() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        let result = StoragePort::find_yak(&storage, "test-yak").unwrap();
        assert_eq!(result, "test-yak");
    }

    #[test]
    fn test_find_yak_fuzzy_match() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        let result = StoragePort::find_yak(&storage, "test").unwrap();
        assert_eq!(result, "test-yak");
    }

    #[test]
    fn test_find_yak_matches_leaf_not_full_path() {
        let storage = InMemoryStorage::new();
        storage.create_yak("parent").unwrap();
        storage.create_yak("parent/child1").unwrap();

        // Should match "parent" yak, not "parent/child1"
        let result = StoragePort::find_yak(&storage, "parent").unwrap();
        assert_eq!(result, "parent");

        // Should match "child1" in "parent/child1"
        let result = StoragePort::find_yak(&storage, "child1").unwrap();
        assert_eq!(result, "parent/child1");
    }

    #[test]
    fn test_find_yak_leaf_only_no_ambiguity() {
        let storage = InMemoryStorage::new();
        storage.create_yak("parent/child1").unwrap();

        // Searching for "parent" should not match "parent/child1"
        let result = StoragePort::find_yak(&storage, "parent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_find_yak_ambiguous() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak1").unwrap();
        storage.create_yak("test-yak2").unwrap();
        let result = StoragePort::find_yak(&storage, "test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ambiguous"));
    }

    #[test]
    fn test_find_yak_not_found() {
        let storage = InMemoryStorage::new();
        let result = StoragePort::find_yak(&storage, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_find_yak_case_insensitive() {
        let storage = InMemoryStorage::new();
        storage.create_yak("Fix the Bug").unwrap();

        let result = StoragePort::find_yak(&storage, "the bug").unwrap();
        assert_eq!(result, "Fix the Bug");
    }

    #[test]
    fn test_write_and_read_field() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        storage
            .write_field("test-yak", "notes", "Field content")
            .unwrap();
        let content = StoragePort::read_field(&storage, "test-yak", "notes").unwrap();
        assert_eq!(content, "Field content");
    }

    #[test]
    fn test_write_field_with_dots() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        storage
            .write_field("test-yak", "notes.txt", "Text file")
            .unwrap();
        let content = StoragePort::read_field(&storage, "test-yak", "notes.txt").unwrap();
        assert_eq!(content, "Text file");
    }

    #[test]
    fn test_read_nonexistent_field() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        let result = StoragePort::read_field(&storage, "test-yak", "nonexistent");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read field"));
    }

    #[test]
    fn test_write_field_nonexistent_yak() {
        let storage = InMemoryStorage::new();
        let result = storage.write_field("nonexistent", "field", "content");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_mark_done_via_state() {
        let storage = InMemoryStorage::new();
        storage.create_yak("test-yak").unwrap();
        storage
            .write_field("test-yak", STATE_FIELD, "done")
            .unwrap();
        let yak = StoragePort::get_yak(&storage, "test-yak").unwrap();
        assert!(yak.is_done());
        assert_eq!(yak.state, "done");
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let storage = InMemoryStorage::new();

        // Create initial yak
        storage.create_yak("yak0").unwrap();

        let mut handles = vec![];

        // Spawn multiple threads that create yaks
        for i in 1..=5 {
            let storage_clone = storage.clone();
            let handle = thread::spawn(move || {
                storage_clone.create_yak(&format!("yak{}", i)).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all yaks were created
        let yaks = StoragePort::list_yaks(&storage).unwrap();
        assert_eq!(yaks.len(), 6);
    }
}
