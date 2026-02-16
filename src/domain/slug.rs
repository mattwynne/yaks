/// Slugify a name: lowercase, spaces to hyphens, strip non-alphanumeric,
/// collapse multiple hyphens. No random suffix — just a human-readable slug.
///
/// Used for directory names on disk. Only needs sibling-uniqueness.
pub fn slugify(name: &str) -> String {
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
    collapsed.trim_matches('-').to_string()
}

/// Generate a unique ID from a human-readable yak name.
///
/// Slug + 4-char random suffix. Immutable once created.
pub fn generate_id(name: &str) -> String {
    let slug = slugify(name);
    let suffix = random_suffix();
    format!("{}-{}", slug, suffix)
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
        assert_eq!(slugify("Make the tea"), "make-the-tea");
    }

    #[test]
    fn slugify_strips_special_characters() {
        assert_eq!(slugify("clean up tests/docs/*"), "clean-up-testsdocs");
    }

    #[test]
    fn slugify_collapses_multiple_hyphens() {
        assert_eq!(slugify("foo - - bar"), "foo-bar");
    }

    #[test]
    fn slugify_is_deterministic() {
        assert_eq!(slugify("test"), slugify("test"));
    }

    #[test]
    fn slugify_preserves_kebab_case() {
        assert_eq!(slugify("fix-the-bug"), "fix-the-bug");
    }

    #[test]
    fn slugify_trims_leading_and_trailing_whitespace() {
        assert_eq!(slugify("  hello world  "), "hello-world");
    }

    #[test]
    fn generate_id_includes_random_suffix() {
        let id = generate_id("Make the tea");
        assert!(
            id.starts_with("make-the-tea-"),
            "Expected id to start with 'make-the-tea-', got '{}'",
            id
        );
        let suffix = &id["make-the-tea-".len()..];
        assert_eq!(suffix.len(), 4);
        assert!(suffix.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn generate_id_produces_different_ids() {
        let id1 = generate_id("test");
        let id2 = generate_id("test");
        assert_ne!(id1, id2);
    }
}
