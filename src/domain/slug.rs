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

/// Generate a deterministic unique ID from a yak name and its
/// ancestry path.
///
/// The ID is `<slug>-<4_char_hash>` where the hash is derived from
/// the full ancestry path: `<grandparent_id>::<parent_id>::<slug>`.
/// For root yaks with no parent, the path is just `<slug>`.
///
/// This ensures:
/// - Same name + same ancestry = same ID (deterministic)
/// - Same name + different parent = different ID (unique)
pub fn generate_id(name: &str, parent_id: Option<&YakId>) -> YakId {
    let slug = slugify(name);
    let ancestry_path = match parent_id {
        Some(pid) => format!("{}::{}", pid, slug),
        None => slug.to_string(),
    };
    let suffix = hash_suffix(&ancestry_path);
    YakId(format!("{}-{}", slug, suffix))
}

fn hash_suffix(input: &str) -> String {
    use std::hash::{Hash, Hasher};

    // Use a fixed-seed hasher for determinism.
    // SipHasher with known keys (0, 0) gives consistent results.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
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
    fn generate_id_includes_hash_suffix() {
        let id = generate_id("Make the tea", None);
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
    fn generate_id_is_deterministic() {
        let id1 = generate_id("test", None);
        let id2 = generate_id("test", None);
        assert_eq!(id1, id2);
    }

    #[test]
    fn generate_id_with_parent_is_deterministic() {
        let parent = YakId::from("project-a1b2");
        let id1 = generate_id("fix-build", Some(&parent));
        let id2 = generate_id("fix-build", Some(&parent));
        assert_eq!(id1, id2);
    }

    #[test]
    fn generate_id_differs_by_parent() {
        let parent_a = YakId::from("project-a-x1y2");
        let parent_b = YakId::from("project-b-z3w4");
        let id_a = generate_id("fix-build", Some(&parent_a));
        let id_b = generate_id("fix-build", Some(&parent_b));
        assert_ne!(id_a, id_b);
        // Both start with the same slug prefix
        assert!(id_a.as_str().starts_with("fix-build-"));
        assert!(id_b.as_str().starts_with("fix-build-"));
    }

    #[test]
    fn generate_id_root_differs_from_child() {
        let parent = YakId::from("project-a1b2");
        let root_id = generate_id("fix-build", None);
        let child_id = generate_id("fix-build", Some(&parent));
        assert_ne!(root_id, child_id);
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

    #[test]
    fn slug_as_ref_str() {
        let slug = Slug::from("make-the-tea");
        let s: &str = slug.as_ref();
        assert_eq!(s, "make-the-tea");

        let empty_slug = Slug::from("");
        assert_eq!(empty_slug.as_ref(), "");

        let slug_with_suffix = Slug::from("xyzzy");
        assert_eq!(slug_with_suffix.as_ref(), "xyzzy");
    }

    #[test]
    fn name_as_ref_str() {
        let name = Name::from("Make the tea");
        let s: &str = name.as_ref();
        assert_eq!(s, "Make the tea");

        let empty_name = Name::from("");
        assert_eq!(empty_name.as_ref(), "");

        let name_with_suffix = Name::from("xyzzy");
        assert_eq!(name_with_suffix.as_ref(), "xyzzy");
    }

    #[test]
    fn name_partial_eq_str() {
        let name = Name::from("hello");
        // Explicitly exercise PartialEq<str> (line 84), not PartialEq<&str> (line 90)
        assert!(<Name as PartialEq<str>>::eq(&name, "hello"));
        assert!(!<Name as PartialEq<str>>::eq(&name, "world"));

        let empty_name = Name::from("");
        assert!(<Name as PartialEq<str>>::eq(&empty_name, ""));
        assert!(!<Name as PartialEq<str>>::eq(&empty_name, "something"));
    }

    #[test]
    fn generate_id_snapshot_root() {
        assert_eq!(
            generate_id("make the tea", None).as_str(),
            "make-the-tea-t3ru"
        );
    }

    #[test]
    fn generate_id_snapshot_child() {
        let parent = generate_id("make the tea", None);
        assert_eq!(
            generate_id("buy biscuits", Some(&parent)).as_str(),
            "buy-biscuits-506d"
        );
    }

    #[test]
    fn name_partial_eq_string() {
        let name = Name::from("hello");
        let hello_string = "hello".to_string();
        let world_string = "world".to_string();

        assert_eq!(name, hello_string);
        assert_ne!(name, world_string);

        let empty_name = Name::from("");
        let empty_string = "".to_string();
        let non_empty_string = "something".to_string();

        assert_eq!(empty_name, empty_string);
        assert_ne!(empty_name, non_empty_string);
    }
}
