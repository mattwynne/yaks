// Yak domain model - a simple value object

use std::collections::HashMap;

use super::slug::{Name, YakId};

const VALID_STATES: &[&str] = &["todo", "wip", "done"];

pub fn validate_state(state: &str) -> Result<(), String> {
    if VALID_STATES.contains(&state) {
        Ok(())
    } else {
        Err(format!(
            "Invalid state '{}'. Valid states are: {}",
            state,
            VALID_STATES.join(", ")
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yak {
    pub id: YakId,
    pub name: Name,
    pub parent_id: Option<YakId>,
    pub state: String,
    pub context: Option<String>,
    pub fields: HashMap<String, String>,
    pub children: Vec<YakId>,
}

impl Yak {
    pub fn is_done(&self) -> bool {
        self.state == "done"
    }
}

/// Validate a yak name provided by the user.
/// Rejects empty names, names containing `/`, and null bytes.
/// Most special characters are allowed because directory names use slugs.
/// Hierarchy is created via --blocks, not by embedding / in names.
pub fn validate_yak_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Yak name cannot be empty".to_string());
    }

    if name.contains('/') {
        return Err(
            "Invalid yak name: '/' is not allowed (use --blocks for hierarchy)".to_string(),
        );
    }

    if name.contains('\0') {
        return Err("Invalid yak name: null bytes are not allowed".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_done_derived_from_state() {
        let yak = Yak {
            id: YakId::from("test"),
            name: Name::from("test"),
            parent_id: None,
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
        };
        assert!(!yak.is_done());

        let done_yak = Yak {
            id: YakId::from("test"),
            name: Name::from("test"),
            parent_id: None,
            state: "done".to_string(),
            context: None,
            fields: HashMap::new(),
            children: vec![],
        };
        assert!(done_yak.is_done());
    }

    #[test]
    fn test_validate_yak_name_valid() {
        assert!(validate_yak_name("test").is_ok());
        assert!(validate_yak_name("dx-rust").is_ok());
    }

    #[test]
    fn test_validate_yak_name_empty() {
        assert!(validate_yak_name("").is_err());
    }

    #[test]
    fn test_validate_yak_name_slash_forbidden() {
        // Slash is forbidden (use --blocks for hierarchy)
        assert!(validate_yak_name("test/name").is_err());
    }

    #[test]
    fn test_validate_yak_name_null_byte_forbidden() {
        assert!(validate_yak_name("test\0name").is_err());
    }

    #[test]
    fn test_validate_yak_name_special_chars_allowed() {
        // These were previously forbidden but are now allowed
        // because directory names use slugs
        assert!(validate_yak_name("test\\name").is_ok());
        assert!(validate_yak_name("test:name").is_ok());
        assert!(validate_yak_name("test*name").is_ok());
        assert!(validate_yak_name("test?name").is_ok());
        assert!(validate_yak_name("test|name").is_ok());
        assert!(validate_yak_name("test<name").is_ok());
        assert!(validate_yak_name("test>name").is_ok());
        assert!(validate_yak_name("test\"name").is_ok());
    }
}
