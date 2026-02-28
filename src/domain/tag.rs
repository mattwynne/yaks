// Tag domain logic - validation, normalization, and formatting

use anyhow::Result;

/// Normalize a tag input: strip leading '@', validate no whitespace.
/// Returns the clean tag name (without @ prefix).
pub fn normalize_tag(input: &str) -> Result<String> {
    let stripped = input.strip_prefix('@').unwrap_or(input);

    if stripped.is_empty() {
        anyhow::bail!("Tag name cannot be empty");
    }

    if stripped.chars().any(|c| c.is_whitespace()) {
        anyhow::bail!("Tag name '{}' cannot contain whitespace", stripped);
    }

    Ok(stripped.to_string())
}

/// Format a tag for display: adds '@' prefix.
pub fn format_tag(tag: &str) -> String {
    format!("@{}", tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_plain_tag() {
        assert_eq!(normalize_tag("v1.0").unwrap(), "v1.0");
    }

    #[test]
    fn normalize_strips_at_prefix() {
        assert_eq!(normalize_tag("@v1.0").unwrap(), "v1.0");
    }

    #[test]
    fn normalize_rejects_whitespace() {
        assert!(normalize_tag("bad tag").is_err());
    }

    #[test]
    fn normalize_rejects_empty() {
        assert!(normalize_tag("").is_err());
    }

    #[test]
    fn normalize_rejects_bare_at() {
        assert!(normalize_tag("@").is_err());
    }

    #[test]
    fn format_adds_at_prefix() {
        assert_eq!(format_tag("v1.0"), "@v1.0");
    }

    #[test]
    fn format_already_clean_tag() {
        assert_eq!(format_tag("needs-review"), "@needs-review");
    }
}
