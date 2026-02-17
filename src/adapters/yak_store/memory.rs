// In-memory storage adapter - for testing only

use crate::domain::field::RESERVED_FIELDS;
use crate::domain::ports::{ReadYakStore, WriteYakStore};
use crate::domain::slug::{Name, YakId};
use crate::domain::{Yak, CONTEXT_FIELD, STATE_FIELD};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct InMemoryStorage {
    // HashMap: yak_name -> HashMap of field_name -> field_content
    yaks: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
    // id -> name mapping for id-based lookups
    id_to_name: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            yaks: Arc::new(RwLock::new(HashMap::new())),
            id_to_name: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Find direct child yak IDs by scanning for entries whose name
    /// is exactly parent_name + "/" + leaf (one level deep).
    fn find_children(&self, parent_name: &str) -> Vec<YakId> {
        let yaks = self.yaks.read().unwrap();
        let id_map = self.id_to_name.read().unwrap();
        self.find_children_from_yaks(&yaks, &id_map, parent_name)
    }

    fn find_children_from_yaks(
        &self,
        yaks: &HashMap<String, HashMap<String, String>>,
        id_map: &HashMap<String, String>,
        parent_name: &str,
    ) -> Vec<YakId> {
        let prefix = format!("{}/", parent_name);
        let name_to_id: HashMap<&String, &String> =
            id_map.iter().map(|(id, name)| (name, id)).collect();

        yaks.keys()
            .filter(|name| {
                if let Some(rest) = name.strip_prefix(&prefix) {
                    !rest.contains('/') // direct child only
                } else {
                    false
                }
            })
            .map(|name| {
                name_to_id
                    .get(name)
                    .map(|id| YakId::from(id.as_str()))
                    .unwrap_or_else(|| YakId::from(name.as_str()))
            })
            .collect()
    }

    /// Resolve a key (name or id) to the yak name used as HashMap key.
    fn resolve_key(&self, key: &str) -> Option<String> {
        let yaks = self.yaks.read().unwrap();
        if yaks.contains_key(key) {
            return Some(key.to_string());
        }
        // Try id→name lookup
        let id_map = self.id_to_name.read().unwrap();
        id_map.get(key).cloned()
    }

    /// Reverse lookup: find the YakId for a given yak name within an id_map.
    fn id_for_name_in(id_map: &HashMap<String, String>, name: &str) -> Option<YakId> {
        id_map
            .iter()
            .find(|(_, v)| **v == name)
            .map(|(k, _)| YakId::from(k.as_str()))
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteYakStore for InMemoryStorage {
    fn create_yak(&self, name: &Name, id: &YakId, parent_id: Option<&YakId>) -> Result<()> {
        // Build path key from parent_id lookup
        let path_key = match parent_id {
            Some(pid) => {
                let id_map = self.id_to_name.read().unwrap();
                let parent_path = id_map
                    .get(pid.as_str())
                    .ok_or_else(|| anyhow::anyhow!("parent '{}' not found", pid))?;
                format!("{}/{}", parent_path, name)
            }
            None => name.as_str().to_string(),
        };

        let mut yaks = self.yaks.write().unwrap();

        if yaks.contains_key(&path_key) {
            anyhow::bail!("Yak '{}' already exists", name);
        }

        let mut fields = HashMap::new();
        // Create empty context.md by default (matching DirectoryStorage behavior)
        fields.insert(CONTEXT_FIELD.to_string(), String::new());
        yaks.insert(path_key.clone(), fields);

        // Store id→name mapping
        if !id.as_str().is_empty() {
            let mut id_map = self.id_to_name.write().unwrap();
            id_map.insert(id.as_str().to_string(), path_key);
        }

        Ok(())
    }

    fn delete_yak(&self, id: &YakId) -> Result<()> {
        let name = self
            .resolve_key(id.as_str())
            .unwrap_or_else(|| id.as_str().to_string());
        let mut yaks = self.yaks.write().unwrap();
        yaks.remove(&name);
        Ok(())
    }

    fn rename_yak(&self, id: &YakId, new_name: &Name) -> Result<()> {
        let from_name = self
            .resolve_key(id.as_str())
            .unwrap_or_else(|| id.as_str().to_string());
        // `new_name` is the new display name, not an existing key -- don't resolve it
        // Reconstruct the path: keep parent prefix, replace leaf
        let new_leaf = new_name
            .as_str()
            .rsplit('/')
            .next()
            .unwrap_or(new_name.as_str());
        let to_name = match from_name.rsplit_once('/') {
            Some((parent, _)) => format!("{}/{}", parent, new_leaf),
            None => new_leaf.to_string(),
        };
        let mut yaks = self.yaks.write().unwrap();

        if !yaks.contains_key(&from_name) {
            anyhow::bail!("yak '{}' not found", id);
        }

        if yaks.contains_key(&to_name) {
            anyhow::bail!("Yak '{}' already exists", new_name);
        }

        if let Some(mut fields) = yaks.remove(&from_name) {
            // Update the name field to reflect the new name
            fields.insert(
                crate::domain::NAME_FIELD.to_string(),
                new_name.as_str().to_string(),
            );
            yaks.insert(to_name.clone(), fields);
        }

        // Update id→name mapping
        let mut id_map = self.id_to_name.write().unwrap();
        if let Some((_id, stored_name)) = id_map.iter_mut().find(|(_, v)| **v == from_name) {
            *stored_name = to_name;
        }

        Ok(())
    }

    fn reparent_yak(&self, id: &YakId, new_parent_id: Option<&YakId>) -> Result<()> {
        // Look up current name-path from id
        let old_name = {
            let id_map = self.id_to_name.read().unwrap();
            id_map
                .get(id.as_str())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?
        };

        // Extract leaf name from the old path
        let leaf = old_name.rsplit('/').next().unwrap_or(&old_name);

        // Build new name-path
        let new_name = match new_parent_id {
            Some(pid) => {
                let id_map = self.id_to_name.read().unwrap();
                let parent_name = id_map
                    .get(pid.as_str())
                    .ok_or_else(|| anyhow::anyhow!("parent '{}' not found", pid))?;
                format!("{}/{}", parent_name, leaf)
            }
            None => leaf.to_string(),
        };

        // Rename in storage
        {
            let mut yaks = self.yaks.write().unwrap();
            if let Some(fields) = yaks.remove(&old_name) {
                yaks.insert(new_name.clone(), fields);
            }
        }

        // Update id→name mapping
        {
            let mut id_map = self.id_to_name.write().unwrap();
            id_map.insert(id.as_str().to_string(), new_name);
        }

        Ok(())
    }

    fn write_field(&self, id: &YakId, field_name: &str, content: &str) -> Result<()> {
        let name = self
            .resolve_key(id.as_str())
            .unwrap_or_else(|| id.as_str().to_string());

        // When updating the name field via id (not via name-path), also rename
        // the HashMap key if the leaf name changed.
        if field_name == crate::domain::NAME_FIELD && name != id.as_str() {
            // id was resolved to a different name, so this is an id-based update
            // Extract leaf from both current key and content (content may be
            // a full path like "parent/child" or just a leaf like "newname")
            let current_leaf = name.rsplit('/').next().unwrap_or(&name);
            let new_leaf = content.rsplit('/').next().unwrap_or(content);
            let new_key = if current_leaf != new_leaf {
                // Reconstruct the path: keep parent prefix, replace leaf
                let parent = name.rsplit_once('/').map(|(p, _)| p);
                match parent {
                    Some(p) => format!("{}/{}", p, new_leaf),
                    None => new_leaf.to_string(),
                }
            } else {
                name.clone()
            };
            if new_key != name {
                let mut yaks = self.yaks.write().unwrap();
                if let Some(fields) = yaks.remove(&name) {
                    let mut updated_fields = fields;
                    updated_fields.insert(field_name.to_string(), content.to_string());
                    yaks.insert(new_key.clone(), updated_fields);
                }
                // Update id→name mapping
                let mut id_map = self.id_to_name.write().unwrap();
                if let Some((_id, stored_name)) = id_map.iter_mut().find(|(_, v)| **v == name) {
                    *stored_name = new_key;
                }
                return Ok(());
            }
        }

        let mut yaks = self.yaks.write().unwrap();
        let fields = yaks
            .get_mut(&name)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", id))?;

        fields.insert(field_name.to_string(), content.to_string());

        Ok(())
    }
}

impl ReadYakStore for InMemoryStorage {
    fn get_yak(&self, id: &YakId) -> Result<Yak> {
        let name = {
            let id_map = self.id_to_name.read().unwrap();
            id_map
                .get(id.as_str())
                .cloned()
                .unwrap_or_else(|| id.as_str().to_string())
        };

        let yaks = self.yaks.read().unwrap();

        let fields = yaks
            .get(&name)
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

        // Collect custom fields (non-reserved)
        let custom_fields: HashMap<String, String> = fields
            .iter()
            .filter(|(k, _)| !RESERVED_FIELDS.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Find children (entries whose name starts with this yak's name + "/")
        let children = self.find_children(&name);

        // Derive parent_id from hierarchical name
        let parent_id = crate::domain::hierarchy::get_parent(&name).and_then(|parent_name| {
            let id_map = self.id_to_name.read().unwrap();
            Self::id_for_name_in(&id_map, &parent_name)
        });

        Ok(Yak {
            id: id.clone(),
            name: Name::from(name),
            parent_id,
            state,
            context,
            fields: custom_fields,
            children,
        })
    }

    fn list_yaks(&self) -> Result<Vec<Yak>> {
        let yaks = self.yaks.read().unwrap();
        let id_map = self.id_to_name.read().unwrap();
        let mut result = Vec::new();

        // Build reverse map: name -> id
        let name_to_id: HashMap<&String, &String> =
            id_map.iter().map(|(id, name)| (name, id)).collect();

        for (name, fields) in yaks.iter() {
            let id = name_to_id
                .get(name)
                .map(|id| YakId::from(id.as_str()))
                .unwrap_or_else(|| YakId::from(name.as_str()));

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

            // Collect custom fields (non-reserved)
            let custom_fields: HashMap<String, String> = fields
                .iter()
                .filter(|(k, _)| !RESERVED_FIELDS.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            // Find children
            let children = self.find_children_from_yaks(&yaks, &id_map, name);

            // Derive parent_id from hierarchical name
            let parent_id = crate::domain::hierarchy::get_parent(name)
                .and_then(|parent_name| Self::id_for_name_in(&id_map, &parent_name));

            result.push(Yak {
                id,
                name: Name::from(name.as_str()),
                parent_id,
                state,
                context,
                fields: custom_fields,
                children,
            });
        }

        // Sort by name for consistent ordering
        result.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(result)
    }

    fn fuzzy_find_yak_id(&self, query: &str) -> Result<YakId> {
        let yaks = self.yaks.read().unwrap();
        let id_map = self.id_to_name.read().unwrap();

        // First, try exact match
        if yaks.contains_key(query) {
            let id = Self::id_for_name_in(&id_map, query).unwrap_or_else(|| YakId::from(query));
            return Ok(id);
        }

        // If not found, try fuzzy match on the leaf node only
        let matches: Vec<&String> = yaks
            .keys()
            .filter(|yak_name| {
                let leaf = yak_name.rsplit('/').next().unwrap_or(yak_name);
                leaf.to_lowercase().contains(&query.to_lowercase())
            })
            .collect();

        match matches.len() {
            0 => anyhow::bail!("yak '{}' not found", query),
            1 => {
                let name = matches[0];
                let id = Self::id_for_name_in(&id_map, name)
                    .unwrap_or_else(|| YakId::from(name.as_str()));
                Ok(id)
            }
            _ => anyhow::bail!("yak name '{}' is ambiguous", query),
        }
    }

    fn read_field(&self, id: &YakId, field_name: &str) -> Result<String> {
        let name = {
            let id_map = self.id_to_name.read().unwrap();
            id_map
                .get(id.as_str())
                .cloned()
                .unwrap_or_else(|| id.as_str().to_string())
        };

        let yaks = self.yaks.read().unwrap();

        let fields = yaks
            .get(&name)
            .ok_or_else(|| anyhow::anyhow!("yak '{}' not found", name))?;

        fields
            .get(field_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Failed to read field '{}' for '{}'", field_name, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_yak_constructs_path_from_parent_id() {
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
        assert!(ReadYakStore::fuzzy_find_yak_id(&storage, "parent/child").is_ok());
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
