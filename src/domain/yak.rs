// YakView - read-model projection of a yak

use std::collections::HashMap;

use super::event_metadata::{Author, Timestamp};
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

/// Read-model projection of a yak.
///
/// This is a DTO (data transfer object) with public fields, used for
/// displaying yak data. It is NOT the core domain entity — see `YakState`
/// (inside `YakMap`) for the authoritative domain type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YakView {
    pub id: YakId,
    pub name: Name,
    pub parent_id: Option<YakId>,
    pub state: String,
    pub context: Option<String>,
    pub fields: HashMap<String, String>,
    pub tags: Vec<String>,
    pub children: Vec<YakId>,
    pub created_by: Author,
    pub created_at: Timestamp,
}

impl YakView {
    pub fn is_done(&self) -> bool {
        self.state == "done"
    }
}

/// Validate a yak name provided by the user.
/// Rejects empty names and null bytes.
/// Most special characters (including `/`) are allowed because
/// directory names use slugs.
pub fn validate_yak_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.trim().is_empty() {
        return Err("Yak name cannot be empty".to_string());
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
        let yak = YakView {
            id: YakId::from("test"),
            name: Name::from("test"),
            parent_id: None,
            state: "todo".to_string(),
            context: None,
            fields: HashMap::new(),
            tags: vec![],
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
        };
        assert!(!yak.is_done());

        let done_yak = YakView {
            id: YakId::from("test"),
            name: Name::from("test"),
            parent_id: None,
            state: "done".to_string(),
            context: None,
            fields: HashMap::new(),
            tags: vec![],
            children: vec![],
            created_by: Author::unknown(),
            created_at: Timestamp::zero(),
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
    fn test_validate_yak_name_slash_allowed() {
        // Slash is allowed (e.g. "fix CI/CD pipeline")
        assert!(validate_yak_name("test/name").is_ok());
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

    #[test]
    fn test_validate_yak_name_whitespace_only() {
        assert!(validate_yak_name("   ").is_err());
        assert!(validate_yak_name("\t").is_err());
        assert!(validate_yak_name("\n").is_err());
        assert!(validate_yak_name("  \t\n  ").is_err());
    }

    #[test]
    fn test_validate_yak_name_with_leading_trailing_whitespace_ok() {
        // Names with whitespace around real content should be fine
        assert!(validate_yak_name("  hello  ").is_ok());
    }
}
