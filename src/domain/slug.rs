use std::fmt;

/// Immutable unique identifier. Created at birth, never changes.
/// Format: slug + 4-char random suffix (e.g., "make-the-tea-a1b2")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct YakId(String);

impl YakId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for YakId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for YakId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for YakId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for YakId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Filesystem-safe name derived from display name.
/// Lowercase, hyphenated, no special chars. Changes on rename.
/// Only needs sibling-uniqueness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slug(String);

impl Slug {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Slug {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Slug {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for Slug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Human-readable display name. Free-form text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Name(String);

impl Name {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Slugify a name: lowercase, spaces to hyphens, strip non-alphanumeric,
/// collapse multiple hyphens. No random suffix — just a human-readable slug.
///
/// Used for directory names on disk. Only needs sibling-uniqueness.
pub fn slugify(name: &str) -> Slug {
    let base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c == ' ' { '-' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();

    // Collapse multiple hyphens
    let mut collapsed = String::new();
    let mut prev_hyphen = false;
    for c in base.chars() {
        if c == '-' {
            if !prev_hyphen {
                collapsed.push(c);
            }
            prev_hyphen = true;
        } else {
            collapsed.push(c);
            prev_hyphen = false;
        }
    }

    // Trim leading/trailing hyphens
    Slug(collapsed.trim_matches('-').to_string())
}

/// Generate a unique ID from a human-readable yak name.
///
/// Slug + 4-char random suffix. Immutable once created.
pub fn generate_id(name: &str) -> YakId {
    let slug = slugify(name);
    let suffix = random_suffix();
    YakId(format!("{}-{}", slug, suffix))
}

fn random_suffix() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u8(0);
    let hash = hasher.finish();

    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    (0..4)
        .map(|i| chars[((hash >> (i * 8)) as usize) % chars.len()])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_lowercases_and_hyphenates() {
        assert_eq!(slugify("Make the tea").as_str(), "make-the-tea");
    }

    #[test]
    fn slugify_strips_special_characters() {
        assert_eq!(
            slugify("clean up tests/docs/*").as_str(),
            "clean-up-testsdocs"
        );
    }

    #[test]
    fn slugify_collapses_multiple_hyphens() {
        assert_eq!(slugify("foo - - bar").as_str(), "foo-bar");
    }

    #[test]
    fn slugify_is_deterministic() {
        assert_eq!(slugify("test"), slugify("test"));
    }

    #[test]
    fn slugify_preserves_kebab_case() {
        assert_eq!(slugify("fix-the-bug").as_str(), "fix-the-bug");
    }

    #[test]
    fn slugify_trims_leading_and_trailing_whitespace() {
        assert_eq!(slugify("  hello world  ").as_str(), "hello-world");
    }

    #[test]
    fn generate_id_includes_random_suffix() {
        let id = generate_id("Make the tea");
        assert!(
            id.as_str().starts_with("make-the-tea-"),
            "Expected id to start with 'make-the-tea-', got '{}'",
            id
        );
        let suffix = &id.as_str()["make-the-tea-".len()..];
        assert_eq!(suffix.len(), 4);
        assert!(suffix.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn generate_id_produces_different_ids() {
        let id1 = generate_id("test");
        let id2 = generate_id("test");
        assert_ne!(id1, id2);
    }

    #[test]
    fn yak_id_display() {
        let id = YakId::from("test-a1b2");
        assert_eq!(format!("{}", id), "test-a1b2");
    }

    #[test]
    fn name_display() {
        let name = Name::from("Make the tea");
        assert_eq!(format!("{}", name), "Make the tea");
    }

    #[test]
    fn slug_display() {
        let slug = Slug::from("make-the-tea");
        assert_eq!(format!("{}", slug), "make-the-tea");
    }

    #[test]
    fn yak_id_from_string() {
        let id = YakId::from("test".to_string());
        assert_eq!(id.as_str(), "test");
    }

    #[test]
    fn name_from_string() {
        let name = Name::from("test".to_string());
        assert_eq!(name.as_str(), "test");
    }

    #[test]
    fn yak_id_as_ref_str() {
        let id = YakId::from("test");
        let s: &str = id.as_ref();
        assert_eq!(s, "test");
    }
}
