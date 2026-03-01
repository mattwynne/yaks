// ListYaks use case - displays all yaks

use crate::domain::slug::{Name, YakId};
use crate::domain::YakView;
// DisplayPort accessed via app.display
use anyhow::Result;
use std::collections::HashMap;

/// Represents a node in the yak hierarchy tree
struct YakNode {
    name: Name,           // Just the leaf name (e.g., "child" not "parent/child")
    full_path: String,    // Full path (e.g., "parent/child")
    yak: Option<YakView>, // None for implicit parents
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
use crate::domain::tag::format_tag;

pub struct ListYaks {
    format: String,
    only: Option<String>,
}

impl ListYaks {
    pub fn new(format: &str, only: Option<&str>) -> Self {
        Self {
            format: format.to_string(),
            only: only.map(|s| s.to_string()),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let format = self.format.as_str();
        let only = self.only.as_deref();
        let yaks = app.store.list_yaks()?;

        // Normalize format (treat "md" and "raw" as aliases)
        let normalized_format = match format {
            "md" => "markdown",
            "raw" => "plain",
            other => other,
        };

        // Validate format
        if !["pretty", "markdown", "plain", "json"].contains(&normalized_format) {
            anyhow::bail!(
                "Unknown format '{}'. Valid formats are: pretty, markdown, plain, json (aliases: md, raw)",
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

        if yaks.is_empty() {
            if normalized_format == "json" {
                app.display.info("[]");
                return Ok(());
            }
            // Only show message in markdown format
            if normalized_format == "markdown" {
                app.display.info("You have no yaks. Are you done?");
            }
            return Ok(());
        }

        // Build hierarchy tree
        let tree = self.build_tree(app, yaks);

        // JSON output: serialize the full tree and return
        if normalized_format == "json" {
            let json_array: Vec<serde_json::Value> = tree.iter().map(node_to_json_value).collect();
            let json_str = serde_json::to_string_pretty(&json_array)
                .map_err(|e| anyhow::anyhow!("Failed to serialize JSON: {}", e))?;
            app.display.info(&json_str);
            return Ok(());
        }

        // Pretty format: top margin
        if normalized_format == "pretty" {
            app.display.info("");
        }

        // Display tree with filtering
        let mut has_output = false;
        let root_prefix = TreePrefix::new();
        self.display_tree(
            app,
            &tree,
            normalized_format,
            only,
            &root_prefix,
            &mut has_output,
        );

        // Pretty format: bottom margin
        if normalized_format == "pretty" && has_output {
            app.display.info("");
        }

        // If filtered and nothing to show
        if !has_output && normalized_format == "markdown" {
            app.display.info("You have no yaks. Are you done?");
        }

        Ok(())
    }

    /// Build a hierarchical tree from flat list of yaks using parent_id
    fn build_tree(&self, _app: &Application, yaks: Vec<YakView>) -> Vec<YakNode> {
        // Index all yak IDs for validation
        let yak_ids: std::collections::HashSet<&str> = yaks.iter().map(|y| y.id.as_str()).collect();

        // Group yaks by parent_id
        let mut children_by_parent: HashMap<Option<&YakId>, Vec<&YakView>> = HashMap::new();
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
            let a_state = a.yak.as_ref().map(|y| y.state.as_str()).unwrap_or("todo");
            let b_state = b.yak.as_ref().map(|y| y.state.as_str()).unwrap_or("todo");

            // Sort: done items first (they're grayed out), then by name
            match (a_state == "done", b_state == "done") {
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

    /// Display tree recursively
    fn display_tree(
        &self,
        app: &Application,
        nodes: &[YakNode],
        format: &str,
        only: Option<&str>,
        prefix: &TreePrefix,
        has_output: &mut bool,
    ) {
        for (i, node) in nodes.iter().enumerate() {
            let is_last = i == nodes.len() - 1;

            // Check if node should be displayed based on filter
            let should_display = self.should_display_node(node, only);

            if should_display {
                *has_output = true;
                self.display_node(app, node, format, prefix, is_last);
            }

            // Recurse to children with updated prefix
            let child_prefix = prefix.for_child(is_last);
            self.display_tree(app, &node.children, format, only, &child_prefix, has_output);
        }
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

    /// Display a single node
    fn display_node(
        &self,
        app: &Application,
        node: &YakNode,
        format: &str,
        prefix: &TreePrefix,
        is_last: bool,
    ) {
        let state = node
            .yak
            .as_ref()
            .map(|y| y.state.as_str())
            .unwrap_or("todo");

        let tags: Vec<String> = node
            .yak
            .as_ref()
            .map(|y| y.tags.iter().map(|t| format_tag(t)).collect())
            .unwrap_or_default();

        match format {
            "plain" => app.display.info(&node.full_path),
            "pretty" => {
                let tree_prefix = if prefix.lines.is_empty() {
                    String::new()
                } else {
                    let ancestor_continuations = &prefix.lines[1..];
                    let connector = if is_last { "╰─ " } else { "├─ " };
                    format!("{}{}", ancestor_continuations.join(""), connector)
                };
                let node_prefix = format!(" {}", tree_prefix);
                app.display
                    .display_yak_pretty(&node_prefix, &node.name, state, &tags);
            }
            _ => {
                let depth = prefix.lines.len();
                app.display
                    .display_yak_markdown(depth, &node.name, state, &tags);
            }
        }
    }
}

/// Recursively convert a YakNode tree to a serde_json::Value
fn node_to_json_value(node: &YakNode) -> serde_json::Value {
    let yak = node.yak.as_ref();

    let id = yak.map(|y| y.id.as_str().to_string()).unwrap_or_default();
    let name = node.name.as_str().to_string();
    let state = yak
        .map(|y| y.state.clone())
        .unwrap_or_else(|| "todo".to_string());
    let context = yak.and_then(|y| y.context.clone());
    let parent_id = yak
        .and_then(|y| y.parent_id.as_ref())
        .map(|p| serde_json::Value::String(p.as_str().to_string()));

    let tags: Vec<&str> = yak
        .map(|y| y.tags.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let fields: serde_json::Map<String, serde_json::Value> = yak
        .map(|y| {
            y.fields
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect()
        })
        .unwrap_or_default();

    let children: Vec<serde_json::Value> = node.children.iter().map(node_to_json_value).collect();

    serde_json::json!({
        "id": id,
        "name": name,
        "state": state,
        "context": context,
        "parent_id": parent_id,
        "tags": tags,
        "fields": fields,
        "children": children,
    })
}

impl UseCase for ListYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
    }
}

/// Recursively build a YakNode and its children from parent_id grouping
fn build_node(
    yak: &YakView,
    children_by_parent: &HashMap<Option<&YakId>, Vec<&YakView>>,
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
        app.handle(ListYaks::new("markdown", Some("done"))).unwrap();
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
        app.handle(ListYaks::new("pretty", Some("done"))).unwrap();
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

        app.handle(ListYaks::new("pretty", None)).unwrap();
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

        app.handle(ListYaks::new("pretty", None)).unwrap();
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
            yak: Some(YakView {
                id: YakId::from(format!("{}-xxxx", name)),
                name: Name::from(name),
                parent_id: None,
                state: state.to_string(),
                context: None,
                fields: HashMap::new(),
                tags: vec![],
                children: vec![],
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
        let list = ListYaks::new("plain", Some("not-done"));
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
        let list = ListYaks::new("plain", Some("not-done"));
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
        app.handle(ListYaks::new("markdown", None)).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("You have no yaks"),
            "Markdown format should show empty message, got: {:?}",
            output
        );

        buffer.clear();

        // Plain should NOT show the message
        app.handle(ListYaks::new("plain", None)).unwrap();
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

        app.handle(ListYaks::new("pretty", None)).unwrap();
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

        let result = app.handle(ListYaks::new("foobar", None));
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

        let result = app.handle(ListYaks::new("pretty", Some("foobar")));
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
        for format in &["pretty", "markdown", "plain", "md", "raw", "json"] {
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

            let result = app.handle(ListYaks::new(format, None));
            assert!(
                result.is_ok(),
                "Format '{}' should be accepted, got error: {:?}",
                format,
                result.unwrap_err()
            );
        }
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

        app.handle(ListYaks::new("pretty", None)).unwrap();
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

        app.handle(ListYaks::new("markdown", None)).unwrap();
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

        app.handle(ListYaks::new("pretty", None)).unwrap();
        let output = buffer.contents();
        assert!(
            !output.contains("@"),
            "Expected no @ in pretty list when no tags, got:\n{output}"
        );
    }
}

#[cfg(test)]
mod json_tests {
    use crate::adapters::user_display::ConsoleDisplay;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::{AddTag, AddYak, Application, ListYaks, SetState};
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
    fn json_empty_list_returns_empty_array() {
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

        app.handle(ListYaks::new("json", None)).unwrap();
        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput:\n{output}"));
        assert_eq!(json, serde_json::json!([]), "Expected empty JSON array");
    }

    #[test]
    fn json_single_yak_has_correct_structure() {
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

        app.handle(AddYak::new("my yak").with_context(Some("some context")))
            .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("json", None)).unwrap();
        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput:\n{output}"));

        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);

        let yak = &arr[0];
        assert_eq!(yak["name"], "my yak");
        assert_eq!(yak["state"], "todo");
        assert_eq!(yak["context"], "some context");
        assert!(yak["id"].as_str().unwrap().contains("my-yak"));
        assert_eq!(yak["parent_id"], serde_json::Value::Null);
        assert_eq!(yak["tags"], serde_json::json!([]));
        assert!(yak["children"].as_array().unwrap().is_empty());
    }

    #[test]
    fn json_nested_yaks_are_recursive() {
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

        app.handle(AddYak::new("parent")).unwrap();
        app.handle(AddYak::new("child").with_parent(Some("parent")))
            .unwrap();
        app.handle(AddYak::new("grandchild").with_parent(Some("child")))
            .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("json", None)).unwrap();
        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput:\n{output}"));

        // Root level should have one yak
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "parent");

        // Parent has one child
        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["name"], "child");

        // Child has one grandchild
        let grandchildren = children[0]["children"].as_array().unwrap();
        assert_eq!(grandchildren.len(), 1);
        assert_eq!(grandchildren[0]["name"], "grandchild");
        assert!(grandchildren[0]["children"].as_array().unwrap().is_empty());
    }

    #[test]
    fn json_includes_tags() {
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

        app.handle(AddYak::new("tagged yak")).unwrap();
        app.handle(AddTag::new(
            "tagged yak",
            vec!["v1".to_string(), "needs-review".to_string()],
        ))
        .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("json", None)).unwrap();
        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput:\n{output}"));

        let tags = json[0]["tags"].as_array().unwrap();
        assert!(
            tags.contains(&serde_json::json!("v1")),
            "Expected v1 in tags: {:?}",
            tags
        );
        assert!(
            tags.contains(&serde_json::json!("needs-review")),
            "Expected needs-review in tags: {:?}",
            tags
        );
    }

    #[test]
    fn json_includes_state() {
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

        app.handle(AddYak::new("wip yak")).unwrap();
        app.handle(SetState::new("wip yak", "wip")).unwrap();
        buffer.clear();

        app.handle(ListYaks::new("json", None)).unwrap();
        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput:\n{output}"));

        assert_eq!(json[0]["state"], "wip");
    }
}
