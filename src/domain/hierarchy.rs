// Hierarchy helper functions for working with parent-child yak names
// Yak names can be slash-separated like "parent/child"

/// Extract parent name from a hierarchical yak name
/// "parent/child" -> Some("parent")
/// "root" -> None
#[allow(dead_code)]
pub fn get_parent(name: &str) -> Option<&str> {
    if let Some(pos) = name.rfind('/') {
        Some(&name[..pos])
    } else {
        None
    }
}

/// Get all ancestors of a name in order from immediate parent to root
/// "a/b/c" -> ["a/b", "a"]
/// "parent/child" -> ["parent"]
/// "root" -> []
#[allow(dead_code)]
pub fn get_ancestors(name: &str) -> Vec<&str> {
    let mut ancestors = Vec::new();
    let mut current = name;

    while let Some(parent) = get_parent(current) {
        ancestors.push(parent);
        current = parent;
    }

    ancestors
}

/// Check if name is a direct child of potential_parent
/// "parent/child" is a child of "parent" -> true
/// "a/b/c" is a child of "a" -> false (a is an ancestor, not direct parent)
/// "a/b/c" is a child of "a/b" -> true
#[allow(dead_code)]
pub fn is_child_of(name: &str, potential_parent: &str) -> bool {
    get_parent(name) == Some(potential_parent)
}

/// Find all direct children of a parent in a HashMap of yak names
#[allow(dead_code)]
pub fn find_children<T>(parent: &str, names: &std::collections::HashMap<String, T>) -> Vec<String> {
    names
        .keys()
        .filter(|name| is_child_of(name, parent))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for get_parent()
    #[test]
    fn get_parent_extracts_parent_from_child() {
        assert_eq!(get_parent("parent/child"), Some("parent"));
    }

    #[test]
    fn get_parent_returns_none_for_root() {
        assert_eq!(get_parent("root"), None);
    }

    #[test]
    fn get_parent_handles_nested_hierarchy() {
        assert_eq!(get_parent("a/b/c"), Some("a/b"));
    }

    #[test]
    fn get_parent_returns_none_for_empty_string() {
        assert_eq!(get_parent(""), None);
    }

    // Tests for get_ancestors()
    #[test]
    fn get_ancestors_returns_all_ancestors() {
        let ancestors = get_ancestors("a/b/c");
        assert_eq!(ancestors, vec!["a/b", "a"]);
    }

    #[test]
    fn get_ancestors_returns_direct_parent_for_child() {
        let ancestors = get_ancestors("parent/child");
        assert_eq!(ancestors, vec!["parent"]);
    }

    #[test]
    fn get_ancestors_returns_empty_for_root() {
        let ancestors = get_ancestors("root");
        assert_eq!(ancestors, Vec::<&str>::new());
    }

    #[test]
    fn get_ancestors_returns_empty_for_empty_string() {
        let ancestors = get_ancestors("");
        assert_eq!(ancestors, Vec::<&str>::new());
    }

    // Tests for is_child_of()
    #[test]
    fn is_child_of_returns_true_for_direct_child() {
        assert!(is_child_of("parent/child", "parent"));
    }

    #[test]
    fn is_child_of_returns_false_for_ancestor() {
        assert!(!is_child_of("a/b/c", "a"));
    }

    #[test]
    fn is_child_of_returns_false_for_unrelated() {
        assert!(!is_child_of("parent/child", "other"));
    }

    #[test]
    fn is_child_of_returns_false_for_root() {
        assert!(!is_child_of("root", "parent"));
    }

    // Tests for find_children()
    #[test]
    fn find_children_returns_direct_children() {
        let mut names = std::collections::HashMap::new();
        names.insert("parent/child1".to_string(), ());
        names.insert("parent/child2".to_string(), ());
        names.insert("other/sibling".to_string(), ());
        names.insert("parent".to_string(), ());

        let children = find_children("parent", &names);
        let mut child_names: Vec<_> = children.iter().map(|s| s.as_str()).collect();
        child_names.sort();

        assert_eq!(child_names, vec!["parent/child1", "parent/child2"]);
    }

    #[test]
    fn find_children_returns_empty_for_no_children() {
        let mut names = std::collections::HashMap::new();
        names.insert("other/child".to_string(), ());

        let children = find_children("parent", &names);
        assert!(children.is_empty());
    }

    #[test]
    fn find_children_ignores_nested_descendants() {
        let mut names = std::collections::HashMap::new();
        names.insert("parent/child".to_string(), ());
        names.insert("parent/child/grandchild".to_string(), ());

        let children = find_children("parent", &names);
        assert_eq!(children.len(), 1);
        assert!(children.iter().any(|c| c == "parent/child"));
    }
}
