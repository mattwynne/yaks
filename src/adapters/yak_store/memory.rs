// In-memory storage adapter - for testing only

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::field::RESERVED_FIELDS;
use crate::domain::ports::{ReadYakStore, WriteYakStore};
use crate::domain::slug::{Name, YakId};
use crate::domain::{Yak, CONTEXT_FIELD, ID_FIELD, NAME_FIELD, STATE_FIELD};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

const PARENT_ID_FIELD: &str = "_parent_id";

#[derive(Clone)]
pub struct InMemoryStorage {
    // HashMap: storage_key -> HashMap of field_name -> field_content
    // storage_key is either the yak id (if non-empty) or the yak name (legacy)
    yaks: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            yaks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Find direct child yak IDs by scanning for entries whose
    /// _parent_id field matches the given id.
    fn find_children_from_yaks(
        yaks: &HashMap<String, HashMap<String, String>>,
        parent_key: &str,
    ) -> Vec<YakId> {
        yaks.iter()
            .filter(|(_, fields)| {
                fields
                    .get(PARENT_ID_FIELD)
                    .map(|pid| pid == parent_key)
                    .unwrap_or(false)
            })
            .map(|(key, _)| YakId::from(key.as_str()))
            .collect()
    }

    /// Resolve a key (name or id) to the storage key used in the HashMap.
    fn resolve_key(&self, key: &str) -> Option<String> {
        let yaks = self.yaks.read().unwrap();
        Self::resolve_key_from_yaks(&yaks, key)
    }

    fn resolve_key_from_yaks(
        yaks: &HashMap<String, HashMap<String, String>>,
        key: &str,
    ) -> Option<String> {
        // Direct key match (id-based or legacy name-based)
        if yaks.contains_key(key) {
            return Some(key.to_string());
        }
        // Try matching by name field
        for (storage_key, fields) in yaks.iter() {
            if let Some(name) = fields.get(NAME_FIELD) {
                if name == key {
                    return Some(storage_key.clone());
                }
            }
        }
        None
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteYakStore for InMemoryStorage {
    fn create_yak(&self, name: &Name, id: &YakId, parent_id: Option<&YakId>) -> Result<()> {
        // Use id as storage key if available, otherwise fall back to name
        let storage_key = if id.as_str().is_empty() {
            name.as_str().to_string()
        } else {
            id.as_str().to_string()
        };

        let mut yaks = self.yaks.write().unwrap();

        if yaks.contains_key(&storage_key) {
            anyhow::bail!("Yak '{}' already exists", name);
        }

        let mut fields = HashMap::new();
        // Create empty context.md by default (matching DirectoryStorage behavior)
        fields.insert(CONTEXT_FIELD.to_string(), String::new());
        // Store the display name
        fields.insert(NAME_FIELD.to_string(), name.as_str().to_string());
        // Store the id (matching DirectoryStorage which writes an id file)
        if !id.as_str().is_empty() {
            fields.insert(ID_FIELD.to_string(), id.as_str().to_string());
        }
        // Store parent_id if present
        if let Some(pid) = parent_id {
            fields.insert(PARENT_ID_FIELD.to_string(), pid.as_str().to_string());
        }
        yaks.insert(storage_key, fields);

        Ok(())
    }

    fn delete_yak(&self, id: &YakId) -> Result<()> {
        let key = self
            .resolve_key(id.as_str())
            .unwrap_or_else(|| id.as_str().to_string());
        let mut yaks = self.yaks.write().unwrap();
        yaks.remove(&key);
        Ok(())
    }

    fn rename_yak(&self, id: &YakId, new_name: &Name) -> Result<()> {
        let key = self
            .resolve_key(id.as_str())
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        let mut yaks = self.yaks.write().unwrap();

        if !yaks.contains_key(&key) {
            anyhow::bail!("yak '{}' not found", id);
        }

        // Check for duplicate name
        for (other_key, other_fields) in yaks.iter() {
            if other_key == &key {
                continue;
            }
            let other_name = other_fields
                .get(NAME_FIELD)
                .map(|s| s.as_str())
                .unwrap_or(other_key.as_str());
            if other_name == new_name.as_str() {
                anyhow::bail!("Yak '{}' already exists", new_name);
            }
        }

        // For legacy yaks (key == name), we need to re-key the HashMap
        let fields = yaks.get(&key).unwrap();
        let is_legacy = !fields.contains_key(NAME_FIELD)
            || fields.get(NAME_FIELD).map(|n| n == &key).unwrap_or(false);

        if is_legacy && key == id.as_str() {
            // Legacy: key is the name, so we need to move the entry
            if let Some(mut fields) = yaks.remove(&key) {
                fields.insert(NAME_FIELD.to_string(), new_name.as_str().to_string());
                yaks.insert(new_name.as_str().to_string(), fields);
            }
        } else {
            // ID-based: just update the name field
            if let Some(fields) = yaks.get_mut(&key) {
                fields.insert(NAME_FIELD.to_string(), new_name.as_str().to_string());
            }
        }

        Ok(())
    }

    fn reparent_yak(&self, id: &YakId, new_parent_id: Option<&YakId>) -> Result<()> {
        let key = self
            .resolve_key(id.as_str())
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        let mut yaks = self.yaks.write().unwrap();

        if let Some(fields) = yaks.get_mut(&key) {
            match new_parent_id {
                Some(pid) => {
                    fields.insert(PARENT_ID_FIELD.to_string(), pid.as_str().to_string());
                }
                None => {
                    fields.remove(PARENT_ID_FIELD);
                }
            }
        }

        Ok(())
    }

    fn write_field(&self, id: &YakId, field_name: &str, content: &str) -> Result<()> {
        let key = self
            .resolve_key(id.as_str())
            .unwrap_or_else(|| id.as_str().to_string());

        let mut yaks = self.yaks.write().unwrap();
        let fields = yaks
            .get_mut(&key)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        fields.insert(field_name.to_string(), content.to_string());

        Ok(())
    }
}

impl ReadYakStore for InMemoryStorage {
    fn get_yak(&self, id: &YakId) -> Result<Yak> {
        let yaks = self.yaks.read().unwrap();
        let key = Self::resolve_key_from_yaks(&yaks, id.as_str())
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        let fields = yaks
            .get(&key)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        // Return leaf name (not full display path)
        let display_name = fields
            .get(NAME_FIELD)
            .cloned()
            .unwrap_or_else(|| key.clone());

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

        // Collect custom fields (non-reserved, excluding internal _parent_id)
        let custom_fields: HashMap<String, String> = fields
            .iter()
            .filter(|(k, _)| {
                !RESERVED_FIELDS.contains(&k.as_str()) && k.as_str() != PARENT_ID_FIELD
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Find children by parent_id
        let children = Self::find_children_from_yaks(&yaks, &key);

        // Read parent_id from fields
        let parent_id = fields
            .get(PARENT_ID_FIELD)
            .map(|pid| YakId::from(pid.as_str()));

        let (created_by, created_at) = fields
            .get(".metadata.json")
            .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok())
            .map(|json| {
                let author = Author {
                    name: json["created_by"]["name"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                    email: json["created_by"]["email"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                };
                let timestamp = Timestamp(json["created_at"].as_i64().unwrap_or(0));
                (author, timestamp)
            })
            .unwrap_or_else(|| (Author::unknown(), Timestamp::zero()));

        Ok(Yak {
            id: YakId::from(key.as_str()),
            name: Name::from(display_name),
            parent_id,
            state,
            context,
            fields: custom_fields,
            children,
            created_by,
            created_at,
        })
    }

    fn list_yaks(&self) -> Result<Vec<Yak>> {
        let yaks = self.yaks.read().unwrap();
        let mut result = Vec::new();

        for (key, fields) in yaks.iter() {
            // Return leaf name (not full display path)
            let display_name = fields
                .get(NAME_FIELD)
                .cloned()
                .unwrap_or_else(|| key.clone());

            let context = fields.get(CONTEXT_FIELD).and_then(|c| {
                if c.is_empty() {
                    None
                } else {
                    Some(c.clone())
                }
            });

            let state = fields
                .get(STATE_FIELD)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "todo".to_string());

            // Collect custom fields (non-reserved, excluding internal _parent_id)
            let custom_fields: HashMap<String, String> = fields
                .iter()
                .filter(|(k, _)| {
                    !RESERVED_FIELDS.contains(&k.as_str()) && k.as_str() != PARENT_ID_FIELD
                })
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            // Find children by parent_id
            let children = Self::find_children_from_yaks(&yaks, key);

            // Read parent_id from fields
            let parent_id = fields
                .get(PARENT_ID_FIELD)
                .map(|pid| YakId::from(pid.as_str()));

            let (created_by, created_at) = fields
                .get(".metadata.json")
                .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok())
                .map(|json| {
                    let author = Author {
                        name: json["created_by"]["name"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string(),
                        email: json["created_by"]["email"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                    };
                    let timestamp = Timestamp(json["created_at"].as_i64().unwrap_or(0));
                    (author, timestamp)
                })
                .unwrap_or_else(|| (Author::unknown(), Timestamp::zero()));

            result.push(Yak {
                id: YakId::from(key.as_str()),
                name: Name::from(display_name),
                parent_id,
                state,
                context,
                fields: custom_fields,
                children,
                created_by,
                created_at,
            });
        }

        // Sort by name for consistent ordering
        result.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(result)
    }

    fn fuzzy_find_yak_id(&self, query: &str) -> Result<YakId> {
        let yaks = self.yaks.read().unwrap();

        // First, try exact key match
        if yaks.contains_key(query) {
            return Ok(YakId::from(query));
        }

        // Try exact name match
        for (key, fields) in yaks.iter() {
            if let Some(name) = fields.get(NAME_FIELD) {
                if name == query {
                    return Ok(YakId::from(key.as_str()));
                }
            }
        }

        // Fuzzy match on name field
        let matches: Vec<&String> = yaks
            .keys()
            .filter(|key| {
                if let Some(fields) = yaks.get(*key) {
                    let name = fields
                        .get(NAME_FIELD)
                        .map(|s| s.as_str())
                        .unwrap_or(key.as_str());
                    name.to_lowercase().contains(&query.to_lowercase())
                } else {
                    false
                }
            })
            .collect();

        match matches.len() {
            0 => anyhow::bail!("yak '{}' not found", query),
            1 => Ok(YakId::from(matches[0].as_str())),
            _ => anyhow::bail!("yak name '{}' is ambiguous", query),
        }
    }

    fn read_field(&self, id: &YakId, field_name: &str) -> Result<String> {
        let yaks = self.yaks.read().unwrap();
        let key = Self::resolve_key_from_yaks(&yaks, id.as_str())
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        let fields = yaks
            .get(&key)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        fields
            .get(field_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Failed to read field '{}' for '{}'", field_name, id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_yak_with_parent_id() {
        let storage = InMemoryStorage::new();
        storage
            .create_yak(&Name::from("parent"), &YakId::from("parent-a1b2"), None)
            .unwrap();
        storage
            .create_yak(
                &Name::from("child"),
                &YakId::from("child-c3d4"),
                Some(&YakId::from("parent-a1b2")),
            )
            .unwrap();
        // Should be findable by name
        assert!(ReadYakStore::fuzzy_find_yak_id(&storage, "child").is_ok());
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let storage = InMemoryStorage::new();

        // Create initial yak
        storage
            .create_yak(&Name::from("yak0"), &YakId::from(""), None)
            .unwrap();

        let mut handles = vec![];

        // Spawn multiple threads that create yaks
        for i in 1..=5 {
            let storage_clone = storage.clone();
            let handle = thread::spawn(move || {
                storage_clone
                    .create_yak(&Name::from(format!("yak{}", i)), &YakId::from(""), None)
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
