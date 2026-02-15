/// Generate a unique slug ID from a human-readable yak name.
///
/// Slugification: lowercase, spaces to hyphens, strip non-alphanumeric
/// (except hyphens), collapse multiple hyphens, append 4-char random suffix.
pub fn generate_slug(name: &str) -> String {
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
    let trimmed = collapsed.trim_matches('-');

    let suffix = random_suffix();
    format!("{}-{}", trimmed, suffix)
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
    fn simple_name_is_lowercased_and_hyphenated() {
        let slug = generate_slug("Make the tea");
        // Should start with "make-the-tea-" followed by 4 alphanumeric chars
        assert!(
            slug.starts_with("make-the-tea-"),
            "Expected slug to start with 'make-the-tea-', got '{}'",
            slug
        );
        let suffix = &slug["make-the-tea-".len()..];
        assert_eq!(
            suffix.len(),
            4,
            "Suffix should be 4 chars, got '{}'",
            suffix
        );
        assert!(
            suffix.chars().all(|c| c.is_ascii_alphanumeric()),
            "Suffix should be alphanumeric, got '{}'",
            suffix
        );
    }

    #[test]
    fn special_characters_are_stripped() {
        let slug = generate_slug("clean up tests/docs/*");
        assert!(
            slug.starts_with("clean-up-testsdocs-"),
            "Expected slug to start with 'clean-up-testsdocs-', got '{}'",
            slug
        );
    }

    #[test]
    fn multiple_hyphens_are_collapsed() {
        let slug = generate_slug("foo - - bar");
        assert!(
            slug.starts_with("foo-bar-"),
            "Expected slug to start with 'foo-bar-', got '{}'",
            slug
        );
    }

    #[test]
    fn each_call_produces_a_different_suffix() {
        let slug1 = generate_slug("test");
        let slug2 = generate_slug("test");
        // Both start with "test-" but have different suffixes
        assert_ne!(slug1, slug2, "Two calls should produce different slugs");
    }

    #[test]
    fn already_kebab_case_is_preserved() {
        let slug = generate_slug("fix-the-bug");
        assert!(
            slug.starts_with("fix-the-bug-"),
            "Expected slug to start with 'fix-the-bug-', got '{}'",
            slug
        );
    }

    #[test]
    fn leading_and_trailing_special_chars_are_trimmed() {
        let slug = generate_slug("  hello world  ");
        assert!(
            slug.starts_with("hello-world-"),
            "Expected slug to start with 'hello-world-', got '{}'",
            slug
        );
    }
}
