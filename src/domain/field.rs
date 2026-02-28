// Field domain logic - validation and reserved field names

use anyhow::Result;

/// Reserved field names that have special meaning
pub const STATE_FIELD: &str = ".state";
pub const CONTEXT_FIELD: &str = ".context.md";
pub const NAME_FIELD: &str = ".name";
pub const ID_FIELD: &str = ".id";
pub const CREATED_FIELD: &str = ".created.json";
pub const PARENT_ID_FIELD: &str = ".parent_id";

/// All reserved field names
pub const RESERVED_FIELDS: &[&str] = &[
    STATE_FIELD,
    CONTEXT_FIELD,
    NAME_FIELD,
    ID_FIELD,
    CREATED_FIELD,
    PARENT_ID_FIELD,
];

/// Validate a field name format (for reading).
///
/// Field names must:
/// - Not be empty
/// - Only contain alphanumeric characters, hyphens, underscores, and dots
/// - Not contain slashes (would create subdirectories)
pub fn validate_field_name_format(field_name: &str) -> Result<()> {
    // Check for empty
    if field_name.is_empty() {
        anyhow::bail!("Field name cannot be empty");
    }

    // Check for valid characters (alphanumeric, hyphens, underscores, dots)
    // No slashes allowed (would create subdirectories)
    if !field_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        anyhow::bail!("Invalid field name '{field_name}' - only letters, numbers, hyphens, underscores, and dots are allowed");
    }

    Ok(())
}

/// Validate a field name for writing (rejects reserved names).
///
/// In addition to format checks, writing is not allowed to
/// reserved field names (state, context.md, name).
pub fn validate_field_name(field_name: &str) -> Result<()> {
    validate_field_name_format(field_name)?;

    // Check for reserved names
    if RESERVED_FIELDS.contains(&field_name) {
        anyhow::bail!("Field name '{field_name}' is reserved");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_field_name_valid() {
        assert!(validate_field_name("notes").is_ok());
        assert!(validate_field_name("priority").is_ok());
        assert!(validate_field_name("notes.txt").is_ok());
        assert!(validate_field_name("my-field").is_ok());
        assert!(validate_field_name("my_field").is_ok());
    }

    #[test]
    fn test_validate_field_name_empty() {
        let result = validate_field_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_validate_field_name_reserved_state() {
        let result = validate_field_name(".state");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn test_validate_field_name_reserved_context() {
        let result = validate_field_name(".context.md");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn test_validate_field_name_reserved_name() {
        let result = validate_field_name(".name");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn test_validate_field_name_reserved_parent_id() {
        let result = validate_field_name(".parent_id");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reserved"));
    }

    #[test]
    fn test_validate_field_name_format_allows_reserved() {
        // Format validation allows reserved names (for reading)
        assert!(validate_field_name_format(".name").is_ok());
        assert!(validate_field_name_format(".state").is_ok());
        assert!(validate_field_name_format(".context.md").is_ok());
    }

    #[test]
    fn test_validate_field_name_invalid_slash() {
        let result = validate_field_name("invalid/name");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid field name"));
    }

    #[test]
    fn test_validate_field_name_invalid_special_chars() {
        assert!(validate_field_name("field:name").is_err());
        assert!(validate_field_name("field*name").is_err());
        assert!(validate_field_name("field?name").is_err());
    }
}

#[cfg(test)]
mod dot_prefix_tests {
    use super::*;

    #[test]
    fn bare_names_no_longer_reserved() {
        // Bare names like "state", "name", "context.md" are now available
        // as user-defined fields since reserved fields are dot-prefixed
        assert!(validate_field_name("state").is_ok());
        assert!(validate_field_name("name").is_ok());
        assert!(validate_field_name("context.md").is_ok());
        assert!(validate_field_name("id").is_ok());
        assert!(validate_field_name("parent_id").is_ok());
    }
}
