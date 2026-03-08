// ListYaks use case - displays all yaks

use crate::domain::slug::{Name, YakId};
use crate::domain::views::{Message, YakTreeNode, YakTreeView};
use crate::domain::{Yak, YakState};
// DisplayPort accessed via app.display
use anyhow::Result;
use std::collections::HashMap;

/// Represents a node in the yak hierarchy tree
struct YakNode {
    name: Name,        // Just the leaf name (e.g., "child" not "parent/child")
    full_path: String, // Full path (e.g., "parent/child")
    yak: Option<Yak>,  // None for implicit parents
    children: Vec<YakNode>,
}

/// Tracks tree drawing state for pretty format
#[derive(Clone)]
struct TreePrefix {
    /// Accumulated prefix lines from parent levels
    lines: Vec<String>,
}

impl TreePrefix {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Create prefix for a child node
    fn for_child(&self, is_last: bool) -> Self {
        let mut new_lines = self.lines.clone();
        let continuation = if is_last { "   " } else { "│  " };
        new_lines.push(continuation.to_string());
        Self { lines: new_lines }
    }
}

use super::{Application, UseCase};
use crate::domain::tag::{format_tag, normalize_tag};

pub struct ListYaks {
    format: String,
    only: Option<String>,
    tag: Option<String>,
}

impl ListYaks {
    pub fn new(format: &str, only: Option<&str>, tag: Option<&str>) -> Self {
        Self {
            format: format.to_string(),
            only: only.map(|s| s.to_string()),
            tag: tag.map(|t| normalize_tag(t).unwrap_or_else(|_| t.to_string())),
        }
    }

    /// Build a hierarchical tree from flat list of yaks using parent_id
    fn build_tree(&self, _app: &Application, yaks: Vec<Yak>) -> Vec<YakNode> {
        // Index all yak IDs for validation
        let yak_ids: std::collections::HashSet<&str> = yaks.iter().map(|y| y.id.as_str()).collect();

        // Group yaks by parent_id
        let mut children_by_parent: HashMap<Option<&YakId>, Vec<&Yak>> = HashMap::new();
        for yak in &yaks {
            // Validate: if parent_id points to a yak not in the list, skip it
            // (corrupted data - but we log and continue rather than crash)
            if let Some(ref pid) = yak.parent_id {
                if !yak_ids.contains(pid.as_str()) {
                    // Orphaned parent_id - treat as root
                    children_by_parent.entry(None).or_default().push(yak);
                    continue;
                }
            }
            children_by_parent
                .entry(yak.parent_id.as_ref())
                .or_default()
                .push(yak);
        }

        // Build tree recursively from roots
        let empty = Vec::new();
        let roots = children_by_parent.get(&None).unwrap_or(&empty);
        let mut root_nodes: Vec<YakNode> = roots
            .iter()
            .map(|yak| build_node(yak, &children_by_parent, ""))
            .collect();

        Self::sort_children(&mut root_nodes);
        root_nodes
    }

    /// Sort children at this level: done first, then not-done, both alphabetically
    fn sort_children(children: &mut [YakNode]) {
        children.sort_by(|a, b| {
            let a_done = a
                .yak
                .as_ref()
                .map(|y| y.state == YakState::Done)
                .unwrap_or(false);
            let b_done = b
                .yak
                .as_ref()
                .map(|y| y.state == YakState::Done)
                .unwrap_or(false);

            // Sort: done items first (they're grayed out), then by name
            match (a_done, b_done) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        // Recursively sort children's children
        for child in children.iter_mut() {
            Self::sort_children(&mut child.children);
        }
    }

    /// Convert internal tree to view model tree, applying filters
    fn build_view_tree(
        &self,
        nodes: &[YakNode],
        only: Option<&str>,
        prefix: &TreePrefix,
    ) -> Vec<YakTreeNode> {
        let mut result = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            let is_last = i == nodes.len() - 1;
            let should_display = self.should_display_node(node, only);

            if should_display {
                let state_str = node
                    .yak
                    .as_ref()
                    .map(|y| y.state.to_string())
                    .unwrap_or_else(|| "todo".to_string());

                let tags: Vec<String> = node
                    .yak
                    .as_ref()
                    .map(|y| y.tags.iter().map(|t| format_tag(t)).collect())
                    .unwrap_or_default();

                let id = node
                    .yak
                    .as_ref()
                    .map(|y| y.id.as_str().to_string())
                    .unwrap_or_default();

                let context = node.yak.as_ref().and_then(|y| y.context.clone());

                let parent_id = node
                    .yak
                    .as_ref()
                    .and_then(|y| y.parent_id.as_ref().map(|p| p.as_str().to_string()));

                let fields = node
                    .yak
                    .as_ref()
                    .map(|y| y.fields.clone())
                    .unwrap_or_default();

                // Compute tree drawing strings
                let (connector, node_prefix) = if prefix.lines.is_empty() {
                    (String::new(), String::new())
                } else {
                    let ancestor_continuations = &prefix.lines[1..];
                    let conn = if is_last {
                        "╰─ ".to_string()
                    } else {
                        "├─ ".to_string()
                    };
                    (conn, ancestor_continuations.join(""))
                };

                let child_prefix = prefix.for_child(is_last);
                let children = self.build_view_tree(&node.children, only, &child_prefix);

                result.push(YakTreeNode {
                    name: node.name.to_string(),
                    full_path: node.full_path.clone(),
                    id,
                    state: state_str,
                    context,
                    parent_id,
                    fields,
                    tags,
                    depth: prefix.lines.len(),
                    connector,
                    prefix: node_prefix,
                    children,
                });
            } else {
                // Even if this node is filtered out, recurse into children
                let child_prefix = prefix.for_child(is_last);
                let mut child_nodes = self.build_view_tree(&node.children, only, &child_prefix);
                result.append(&mut child_nodes);
            }
        }
        result
    }

    /// Check if node matches the filter
    fn should_display_node(&self, node: &YakNode, only: Option<&str>) -> bool {
        match only {
            Some("done") => node.yak.as_ref().map(|y| y.is_done()).unwrap_or(false),
            Some("not-done") => {
                !node.yak.as_ref().map(|y| y.is_done()).unwrap_or(false) || node.yak.is_none()
            }
            _ => true,
        }
    }
}

impl UseCase for ListYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let format = self.format.as_str();
        let only = self.only.as_deref();
        let mut yaks = app.store.list_yaks()?;

        // Normalize format
        let normalized_format = match format {
            "md" => "markdown",
            "raw" => "plain",
            other => other,
        };

        // Validate format
        if !["pretty", "markdown", "plain", "ids"].contains(&normalized_format) {
            anyhow::bail!(
                "Unknown format '{}'. Valid formats are: pretty, markdown, plain, ids (aliases: md, raw)",
                format
            );
        }

        // Validate filter
        if let Some(filter) = only {
            if !["done", "not-done"].contains(&filter) {
                anyhow::bail!(
                    "Unknown filter '{}'. Valid filters are: done, not-done",
                    filter
                );
            }
        }

        // Apply tag filter
        if let Some(ref tag) = self.tag {
            yaks.retain(|y| y.tags.contains(tag));
        }

        // Handle ids format early (before tree building)
        if normalized_format == "ids" {
            for yak in &yaks {
                app.display
                    .message(&Message::Info(yak.id.as_str().to_string()));
            }
            return Ok(());
        }

        // Build hierarchy tree (even if empty)
        let tree = if yaks.is_empty() {
            vec![]
        } else {
            self.build_tree(app, yaks)
        };

        // Build YakTreeView from internal tree
        let view_nodes = self.build_view_tree(&tree, only, &TreePrefix::new());

        // For markdown format when empty, show a message instead of the list
        if tree.is_empty() && normalized_format == "markdown" {
            app.display
                .message(&Message::Info("You have no yaks. Are you done?".into()));
            return Ok(());
        }

        let view = YakTreeView {
            nodes: view_nodes,
            format: normalized_format.to_string(),
            is_empty: false,
        };

        app.display.show_list(&view);
        Ok(())
    }
}

/// Recursively build a YakNode and its children from parent_id grouping
fn build_node(
    yak: &Yak,
    children_by_parent: &HashMap<Option<&YakId>, Vec<&Yak>>,
    parent_path: &str,
) -> YakNode {
    let leaf_name = yak.name.as_str();
    let full_path = if parent_path.is_empty() {
        leaf_name.to_string()
    } else {
        format!("{}/{}", parent_path, leaf_name)
    };

    let empty = Vec::new();
    let child_yaks = children_by_parent.get(&Some(&yak.id)).unwrap_or(&empty);
    let children: Vec<YakNode> = child_yaks
        .iter()
        .map(|child| build_node(child, children_by_parent, &full_path))
        .collect();

    YakNode {
        name: Name::from(leaf_name),
        full_path,
        yak: Some(yak.clone()),
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::user_display::ConsoleDisplay;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::{AddYak, Application, SetState};
    use crate::infrastructure::EventBus;

    fn make_app<'a>(
        event_store: &'a mut InMemoryEventStore,
        event_bus: &'a mut EventBus,
        storage: &'a InMemoryStorage,
        display: &'a ConsoleDisplay,
        input: &'a InMemoryInput,
        auth: &'a InMemoryAuthentication,
    ) -> Application<'a> {
        Application::new(event_store, event_bus, storage, display, input, None, auth)
    }

    // Mutant 1 (line 89): only markdown format shows "You have no yaks"
    // when a filter produces no results. Pretty format should stay silent.
    #[test]
    fn filtered_list_shows_no_yaks_message_only_in_markdown() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        // Add a yak that is NOT done so the "done" filter produces no output
        app.handle(AddYak::new("pending-yak")).unwrap();
        buffer.clear();

        // Markdown format: should emit the "no yaks" message
        app.handle(ListYaks::new("markdown", Some("done"), None))
            .unwrap();
        let output = buffer.contents();
        let markdown_lines: Vec<&str> = output.lines().collect();
        assert!(
            markdown_lines
                .iter()
                .any(|m| m.contains("You have no yaks")),
            "Markdown format should show 'You have no yaks' when filter has no results, got: {:?}",
            markdown_lines
        );

        buffer.clear();

        // Pretty format: should NOT emit the "no yaks" message
        app.handle(ListYaks::new("pretty", Some("done"), None))
            .unwrap();
        let output = buffer.contents();
        let pretty_lines: Vec<&str> = output.lines().collect();
        assert!(
            !pretty_lines.iter().any(|m| m.contains("You have no yaks")),
            "Pretty format should NOT show 'You have no yaks', got: {:?}",
            pretty_lines
        );
    }

    // Mutant 2 (line 140): done items sort before non-done items in pretty output
    #[test]
    fn done_yaks_sort_before_not_done_yaks() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        // Add two yaks; "beta" will be done, "alpha" will remain todo
        // Use names that would sort "alpha" before "beta" alphabetically
        // so any name-only sorting would put alpha first
        app.handle(AddYak::new("alpha")).unwrap();
        app.handle(AddYak::new("beta")).unwrap();
        app.handle(SetState::new("beta", "done")).unwrap();
        buffer.clear();

        app.handle(ListYaks::new("pretty", None, None)).unwrap();
        let output = buffer.contents();
        let messages: Vec<&str> = output.lines().collect();

        // Find positions of alpha and beta in the output
        let beta_pos = messages.iter().position(|m| m.contains("beta"));
        let alpha_pos = messages.iter().position(|m| m.contains("alpha"));

        assert!(
            beta_pos.is_some() && alpha_pos.is_some(),
            "Both yaks should appear in the output, got: {:?}",
            messages
        );
        assert!(
            beta_pos.unwrap() < alpha_pos.unwrap(),
            "Done yak 'beta' should appear before non-done 'alpha', got: {:?}",
            messages
        );
    }

    // Children show tree connectors: ├─ for non-last, ╰─ for last
    #[test]
    fn tree_connectors_distinguish_last_from_non_last_child() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        // Parent with two children
        app.handle(AddYak::new("parent")).unwrap();
        app.handle(AddYak::new("aaa").with_parent(Some("parent")))
            .unwrap();
        app.handle(AddYak::new("zzz").with_parent(Some("parent")))
            .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("pretty", None, None)).unwrap();
        let output = buffer.contents();
        let messages: Vec<&str> = output.lines().collect();

        let parent_line = messages.iter().find(|m| m.contains("parent"));
        let aaa_line = messages.iter().find(|m| m.contains("aaa"));
        let zzz_line = messages.iter().find(|m| m.contains("zzz"));

        assert!(parent_line.is_some() && aaa_line.is_some() && zzz_line.is_some());

        // Pretty format has 1-char whitespace margin all around
        assert!(
            messages.first().unwrap().is_empty(),
            "Expected leading blank line for top margin, got: {:?}",
            messages
        );
        assert!(
            messages.last().unwrap().is_empty(),
            "Expected trailing blank line for bottom margin, got: {:?}",
            messages
        );

        // Root has space prefix (left margin)
        assert!(
            parent_line.unwrap().starts_with(" ○"),
            "Root should have space prefix, got: {:?}",
            parent_line
        );

        // Non-last child gets space + ├─ connector
        assert!(
            aaa_line.unwrap().starts_with(" ├─ ○"),
            "Non-last child 'aaa' should have ' ├─' connector, got: {:?}",
            aaa_line
        );
        // Last child gets space + ╰─ connector
        assert!(
            zzz_line.unwrap().starts_with(" ╰─ ○"),
            "Last child 'zzz' should have ' ╰─' connector, got: {:?}",
            zzz_line
        );
    }

    use crate::domain::event_metadata::{Author, Timestamp};

    fn make_yak_node(name: &str, state: &str) -> YakNode {
        YakNode {
            name: Name::from(name),
            full_path: name.to_string(),
            yak: Some(Yak {
                id: YakId::from(format!("{}-xxxx", name)),
                name: Name::from(name),
                parent_id: None,
                state: state.parse::<YakState>().unwrap(),
                context: None,
                fields: HashMap::new(),
                tags: vec![],
                created_by: Author::unknown(),
                created_at: Timestamp::zero(),
            }),
            children: vec![],
        }
    }

    // Line 140: sort_children must put not-done items after done items,
    // even when the not-done item sorts alphabetically before the done one.
    // Input order is [done, not-done] to force the sort comparator to
    // exercise the (false, true) match arm.
    #[test]
    fn sort_children_not_done_sorts_after_done() {
        let mut nodes = vec![
            make_yak_node("bbb", "done"), // done, alphabetically second
            make_yak_node("aaa", "todo"), // not-done, alphabetically first
        ];
        ListYaks::sort_children(&mut nodes);
        assert_eq!(
            nodes[0].name.as_str(),
            "bbb",
            "Done item should sort before not-done item"
        );
        assert_eq!(
            nodes[1].name.as_str(),
            "aaa",
            "Not-done item should sort after done item"
        );
    }

    // Line 182: not-done filter must exclude done yaks (not fall through to _ => true)
    #[test]
    fn not_done_filter_excludes_done_yaks() {
        let list = ListYaks::new("plain", Some("not-done"), None);
        let node = make_yak_node("finished", "done");
        assert!(
            !list.should_display_node(&node, Some("not-done")),
            "Done yak should be excluded by not-done filter"
        );
    }

    // Lines 183: not-done filter must include not-done yaks
    // Catches both the `!` deletion and `||` to `&&` mutants
    #[test]
    fn not_done_filter_includes_not_done_yaks() {
        let list = ListYaks::new("plain", Some("not-done"), None);
        let node = make_yak_node("pending", "todo");
        assert!(
            list.should_display_node(&node, Some("not-done")),
            "Not-done yak should be included by not-done filter"
        );
    }

    // Line 67: empty yak list shows message only in markdown format
    #[test]
    fn empty_list_shows_message_only_in_markdown() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        // No yaks added - list is empty

        // Markdown should show the message
        app.handle(ListYaks::new("markdown", None, None)).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("You have no yaks"),
            "Markdown format should show empty message, got: {:?}",
            output
        );

        buffer.clear();

        // Plain should NOT show the message
        app.handle(ListYaks::new("plain", None, None)).unwrap();
        let output = buffer.contents();
        assert!(
            !output.contains("You have no yaks"),
            "Plain format should not show empty message, got: {:?}",
            output
        );
    }

    // Grandchild shows ancestor continuation lines
    #[test]
    fn grandchild_shows_ancestor_continuation_lines() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        // root -> branch (not last) -> leaf, plus sibling (last)
        app.handle(AddYak::new("root")).unwrap();
        app.handle(AddYak::new("branch").with_parent(Some("root")))
            .unwrap();
        app.handle(AddYak::new("leaf").with_parent(Some("branch")))
            .unwrap();
        app.handle(AddYak::new("sibling").with_parent(Some("root")))
            .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("pretty", None, None)).unwrap();
        let output = buffer.contents();
        let messages: Vec<&str> = output.lines().collect();

        let leaf_line = messages.iter().find(|m| m.contains("leaf"));
        assert!(
            leaf_line.is_some(),
            "Expected 'leaf' in output: {:?}",
            messages
        );

        // Leaf under non-last branch should show space + │ continuation + ╰─ connector
        assert!(
            leaf_line.unwrap().starts_with(" │  ╰─ ○"),
            "Grandchild under non-last parent should have space + │ continuation, got: {:?}",
            leaf_line
        );
    }

    #[test]
    fn invalid_format_returns_error() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, _buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        let result = app.handle(ListYaks::new("foobar", None, None));
        assert!(result.is_err(), "Expected error for invalid format");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unknown format"),
            "Expected 'Unknown format' in error, got: {}",
            err
        );
        assert!(
            err.contains("pretty"),
            "Expected valid formats listed in error, got: {}",
            err
        );
    }

    #[test]
    fn invalid_only_filter_returns_error() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, _buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        let result = app.handle(ListYaks::new("pretty", Some("foobar"), None));
        assert!(result.is_err(), "Expected error for invalid filter");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Unknown filter"),
            "Expected 'Unknown filter' in error, got: {}",
            err
        );
        assert!(
            err.contains("done"),
            "Expected valid filters listed in error, got: {}",
            err
        );
    }

    #[test]
    fn valid_formats_accepted() {
        for format in &["pretty", "markdown", "plain", "md", "raw", "ids"] {
            let mut event_store = InMemoryEventStore::new();
            let mut event_bus = EventBus::new();
            let storage = InMemoryStorage::new();
            event_bus.register(Box::new(storage.clone()));
            let (display, _buffer) = make_test_display();
            let input = InMemoryInput::new();
            let auth = InMemoryAuthentication::new();
            let mut app = make_app(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                &auth,
            );

            let result = app.handle(ListYaks::new(format, None, None));
            assert!(
                result.is_ok(),
                "Format '{}' should be accepted, got error: {:?}",
                format,
                result.unwrap_err()
            );
        }
    }

    // Line 141: pretty format bottom margin requires BOTH pretty format AND has_output.
    // When a filter excludes all yaks, has_output stays false, so no trailing blank line.
    // This kills the mutant: replace `&&` with `||` on the bottom-margin condition.
    #[test]
    fn pretty_format_no_trailing_blank_when_filter_excludes_all() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        // Add a yak in "todo" state
        app.handle(AddYak::new("todo yak")).unwrap();
        buffer.clear();

        // List with pretty format, filtering by "done" — no nodes match
        app.handle(ListYaks::new("pretty", Some("done"), None))
            .unwrap();
        let output = buffer.contents();

        // With correct code (&&): only top margin blank line is emitted,
        // because has_output is false so bottom margin is skipped.
        // With mutant (||): both top AND bottom margin blank lines are emitted.
        // So we assert the output is exactly one blank line (the top margin only).
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "Expected only the top-margin blank line when filter excludes all yaks, got {} lines: {:?}",
            lines.len(),
            lines
        );
        assert_eq!(
            lines[0], "",
            "Expected the single line to be the top-margin blank, got: {:?}",
            lines[0]
        );
    }
}

#[cfg(test)]
mod tag_tests {
    use crate::adapters::user_display::ConsoleDisplay;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::{AddTag, AddYak, Application, ListYaks};
    use crate::infrastructure::EventBus;

    fn make_app<'a>(
        event_store: &'a mut InMemoryEventStore,
        event_bus: &'a mut EventBus,
        storage: &'a InMemoryStorage,
        display: &'a ConsoleDisplay,
        input: &'a InMemoryInput,
        auth: &'a InMemoryAuthentication,
    ) -> Application<'a> {
        Application::new(event_store, event_bus, storage, display, input, None, auth)
    }

    #[test]
    fn pretty_list_shows_tags_inline() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        app.handle(AddYak::new("my yak")).unwrap();
        app.handle(AddTag::new("my yak", vec!["v1.0".to_string()]))
            .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("pretty", None, None)).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("@v1.0"),
            "Expected @v1.0 in pretty list output, got:\n{output}"
        );
    }

    #[test]
    fn markdown_list_shows_tags_inline() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        app.handle(AddYak::new("my yak")).unwrap();
        app.handle(AddTag::new(
            "my yak",
            vec!["v1.0".to_string(), "needs-review".to_string()],
        ))
        .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("markdown", None, None)).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("@v1.0"),
            "Expected @v1.0 in markdown list output, got:\n{output}"
        );
        assert!(
            output.contains("@needs-review"),
            "Expected @needs-review in markdown list output, got:\n{output}"
        );
    }

    #[test]
    fn pretty_list_without_tags_has_no_at() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &auth,
        );

        app.handle(AddYak::new("my yak")).unwrap();
        buffer.clear();

        app.handle(ListYaks::new("pretty", None, None)).unwrap();
        let output = buffer.contents();
        assert!(
            !output.contains("@"),
            "Expected no @ in pretty list when no tags, got:\n{output}"
        );
    }
}
