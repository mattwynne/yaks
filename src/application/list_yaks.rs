// ListYaks use case - displays all yaks

use crate::domain::slug::{Name, YakId};
use crate::domain::Yak;
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

        if yaks.is_empty() {
            // Only show message in markdown format
            if normalized_format == "markdown" {
                app.display.info("You have no yaks. Are you done?");
            }
            return Ok(());
        }

        // Build hierarchy tree
        let tree = self.build_tree(app, yaks);

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

        // If filtered and nothing to show
        if !has_output && normalized_format == "markdown" {
            app.display.info("You have no yaks. Are you done?");
        }

        Ok(())
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

        match format {
            "plain" => app.display.info(&node.full_path),
            "pretty" => {
                let node_prefix = if prefix.lines.is_empty() {
                    String::new()
                } else {
                    let ancestor_continuations = &prefix.lines[1..];
                    let connector = if is_last { "╰─ " } else { "├─ " };
                    format!("{}{}", ancestor_continuations.join(""), connector)
                };
                app.display
                    .display_yak_pretty(&node_prefix, &node.name, state);
            }
            _ => {
                let depth = prefix.lines.len();
                app.display.display_yak_markdown(depth, &node.name, state);
            }
        }
    }
}

impl UseCase for ListYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
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
    use crate::adapters::{
        InMemoryAuthentication, InMemoryDisplay, InMemoryEventStore, InMemoryInput, InMemoryStorage,
    };
    use crate::application::{AddYak, Application, SetState};
    use crate::infrastructure::EventBus;

    fn make_app<'a>(
        event_store: &'a mut InMemoryEventStore,
        event_bus: &'a mut EventBus,
        storage: &'a InMemoryStorage,
        display: &'a InMemoryDisplay,
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
        let display = InMemoryDisplay::new();
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
        display.clear();

        // Markdown format: should emit the "no yaks" message
        app.handle(ListYaks::new("markdown", Some("done"))).unwrap();
        let markdown_messages = display.get_info_messages();
        assert!(
            markdown_messages
                .iter()
                .any(|m| m.contains("You have no yaks")),
            "Markdown format should show 'You have no yaks' when filter has no results, got: {:?}",
            markdown_messages
        );

        display.clear();

        // Pretty format: should NOT emit the "no yaks" message
        app.handle(ListYaks::new("pretty", Some("done"))).unwrap();
        let pretty_messages = display.get_info_messages();
        assert!(
            !pretty_messages
                .iter()
                .any(|m| m.contains("You have no yaks")),
            "Pretty format should NOT show 'You have no yaks', got: {:?}",
            pretty_messages
        );
    }

    // Mutant 2 (line 140): done items sort before non-done items in pretty output
    #[test]
    fn done_yaks_sort_before_not_done_yaks() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let display = InMemoryDisplay::new();
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
        display.clear();

        app.handle(ListYaks::new("pretty", None)).unwrap();
        let messages = display.get_info_messages();

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
        let display = InMemoryDisplay::new();
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
        display.clear();

        app.handle(ListYaks::new("pretty", None)).unwrap();
        let messages = display.get_info_messages();

        let parent_line = messages.iter().find(|m| m.contains("parent"));
        let aaa_line = messages.iter().find(|m| m.contains("aaa"));
        let zzz_line = messages.iter().find(|m| m.contains("zzz"));

        assert!(parent_line.is_some() && aaa_line.is_some() && zzz_line.is_some());

        // Root has no prefix
        assert!(
            parent_line.unwrap().starts_with("○"),
            "Root should have no prefix, got: {:?}",
            parent_line
        );

        // Non-last child gets ├─ connector
        assert!(
            aaa_line.unwrap().starts_with("├─ ○"),
            "Non-last child 'aaa' should have ├─ connector, got: {:?}",
            aaa_line
        );
        // Last child gets ╰─ connector
        assert!(
            zzz_line.unwrap().starts_with("╰─ ○"),
            "Last child 'zzz' should have ╰─ connector, got: {:?}",
            zzz_line
        );
    }

    // Grandchild shows ancestor continuation lines
    #[test]
    fn grandchild_shows_ancestor_continuation_lines() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let display = InMemoryDisplay::new();
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
        display.clear();

        app.handle(ListYaks::new("pretty", None)).unwrap();
        let messages = display.get_info_messages();

        let leaf_line = messages.iter().find(|m| m.contains("leaf"));
        assert!(
            leaf_line.is_some(),
            "Expected 'leaf' in output: {:?}",
            messages
        );

        // Leaf under non-last branch should show │ continuation + ╰─ connector
        assert!(
            leaf_line.unwrap().starts_with("│  ╰─ ○"),
            "Grandchild under non-last parent should have │ continuation, got: {:?}",
            leaf_line
        );
    }
}
