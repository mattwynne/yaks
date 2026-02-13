// Yak domain model - a simple value object

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
    pub name: String,
    pub state: String,
    pub context: Option<String>,
}

impl Yak {
    pub fn is_done(&self) -> bool {
        self.state == "done"
    }
}

/// Validate a yak name
/// Rejects names containing forbidden characters: \ : * ? | < > "
/// Slashes (/) are allowed for hierarchical yaks (e.g., "dx/rust")
pub fn validate_yak_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Yak name cannot be empty".to_string());
    }

    // Check for forbidden characters (matches bash version)
    // Forbidden: \ : * ? | < > "
    // Allowed: / (for hierarchy)
    const FORBIDDEN_CHARS: &[char] = &['\\', ':', '*', '?', '|', '<', '>', '"'];

    for c in FORBIDDEN_CHARS {
        if name.contains(*c) {
            return Err(
                "Invalid yak name: contains forbidden characters (\\ : * ? | < > \")".to_string(),
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_done_derived_from_state() {
        let yak = Yak {
            name: "test".to_string(),
            state: "todo".to_string(),
            context: None,
        };
        assert!(!yak.is_done());

        let done_yak = Yak {
            name: "test".to_string(),
            state: "done".to_string(),
            context: None,
        };
        assert!(done_yak.is_done());
    }

    #[test]
    fn test_validate_yak_name_valid() {
        assert!(validate_yak_name("test").is_ok());
        assert!(validate_yak_name("dx/rust").is_ok());
    }

    #[test]
    fn test_validate_yak_name_empty() {
        assert!(validate_yak_name("").is_err());
    }

    #[test]
    fn test_validate_yak_name_forbidden_chars() {
        // Test each forbidden character
        assert!(validate_yak_name("test\\name").is_err());
        assert!(validate_yak_name("test:name").is_err());
        assert!(validate_yak_name("test*name").is_err());
        assert!(validate_yak_name("test?name").is_err());
        assert!(validate_yak_name("test|name").is_err());
        assert!(validate_yak_name("test<name").is_err());
        assert!(validate_yak_name("test>name").is_err());
        assert!(validate_yak_name("test\"name").is_err());

        // Slash should be allowed (for hierarchy)
        assert!(validate_yak_name("test/name").is_ok());
    }
}
