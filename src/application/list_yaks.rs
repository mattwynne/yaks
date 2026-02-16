// ListYaks use case - displays all yaks

use crate::domain::Yak;
// DisplayPort accessed via app.display
use anyhow::Result;
use std::collections::HashMap;

/// Represents a node in the yak hierarchy tree
struct YakNode {
    name: String,      // Just the leaf name (e.g., "child" not "parent/child")
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

    /// Build a hierarchical tree from flat list of yaks
    fn build_tree(&self, _app: &Application, yaks: Vec<Yak>) -> Vec<YakNode> {
        let mut nodes_by_path: HashMap<String, YakNode> = HashMap::new();

        // First pass: create nodes for all yaks and implicit parents
        for yak in &yaks {
            let name_str = yak.name.as_str();
            let parts: Vec<&str> = name_str.split('/').collect();

            // Create implicit parent nodes if they don't exist
            for i in 1..parts.len() {
                let parent_path = parts[..i].join("/");
                if !nodes_by_path.contains_key(&parent_path) {
                    let parent_name = parts[i - 1].to_string();
                    nodes_by_path.insert(
                        parent_path.clone(),
                        YakNode {
                            name: parent_name,
                            full_path: parent_path.clone(),
                            yak: None, // Implicit parent (no actual yak)
                            children: Vec::new(),
                        },
                    );
                }
            }

            // Create node for this yak
            let name = parts.last().unwrap_or(&"").to_string();
            nodes_by_path.insert(
                yak.name.to_string(),
                YakNode {
                    name,
                    full_path: yak.name.to_string(),
                    yak: Some(yak.clone()),
                    children: Vec::new(),
                },
            );
        }

        // Second pass: build parent-child relationships
        // Sort paths by depth (deepest first) to ensure children are processed before parents
        let mut all_paths: Vec<String> = nodes_by_path.keys().cloned().collect();
        all_paths.sort_by_key(|p| std::cmp::Reverse(p.matches('/').count()));

        // Extract children from deepest to shallowest
        for path in &all_paths {
            let parts: Vec<&str> = path.split('/').collect();

            if parts.len() == 1 {
                // Root node - leave it
                continue;
            }

            // Child node - attach to parent
            let parent_path = parts[..parts.len() - 1].join("/");

            // Remove child from map and attach to parent
            if let Some(child_node) = nodes_by_path.remove(path) {
                if let Some(parent_node) = nodes_by_path.get_mut(&parent_path) {
                    parent_node.children.push(child_node);
                } else {
                    // This shouldn't happen since we created all parents in first pass
                    // But if it does, put the node back
                    nodes_by_path.insert(path.clone(), child_node);
                }
            }
        }

        // Extract root nodes and sort
        let mut roots: Vec<YakNode> = nodes_by_path
            .into_iter()
            .filter(|(path, _)| !path.contains('/'))
            .map(|(_, node)| node)
            .collect();

        Self::sort_children(&mut roots);
        roots
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
