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
                    "  ".to_string()
                } else if prefix.lines.len() == 1 {
                    let connector = if is_last { "╰─ " } else { "├─ " };
                    format!("  {}", connector)
                } else {
                    let ancestor_continuations = &prefix.lines[1..];
                    let connector = if is_last { "╰─ " } else { "├─ " };
                    format!("  {}{}", ancestor_continuations.join(""), connector)
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
