// ListYaks use case - displays all yaks

use crate::adapters::views::{Message, ReadinessView, YakBlockerView, YakTreeNode, YakTreeView};
use crate::application::readiness::build_readiness_views;
use crate::domain::slug::{Name, YakId};
use crate::domain::{Yak, YakState};
// DisplayPort accessed via app.display
use anyhow::Result;
use std::collections::{HashMap, HashSet};

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

struct ViewBuildContext<'a> {
    only: Option<&'a str>,
    tag: Option<&'a str>,
    ready: bool,
    readiness_by_id: &'a HashMap<YakId, ReadinessView>,
    blockers_by_id: &'a HashMap<YakId, Vec<YakBlockerView>>,
    visible_ids: Option<&'a HashSet<YakId>>,
}

impl TreePrefix {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Create prefix for a child node
    fn for_child(&self, is_last_in_full_tree: bool, hidden_siblings_at_level: bool) -> Self {
        let mut new_lines = self.lines.clone();
        let continuation = if hidden_siblings_at_level {
            "┆  "
        } else if is_last_in_full_tree {
            "   "
        } else {
            "│  "
        };
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
    ready: bool,
}

impl ListYaks {
    pub fn new(format: &str, only: Option<&str>, tag: Option<&str>) -> Self {
        Self {
            format: format.to_string(),
            only: only.map(|s| s.to_string()),
            tag: tag.map(|t| normalize_tag(t).unwrap_or_else(|_| t.to_string())),
            ready: false,
        }
    }

    pub fn with_ready(mut self, ready: bool) -> Self {
        self.ready = ready;
        self
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
        context: &ViewBuildContext<'_>,
        prefix: &TreePrefix,
    ) -> Vec<YakTreeNode> {
        let mut result = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            let is_last_in_full_tree = i == nodes.len() - 1;
            let is_visible_by_focus = self.is_visible_by_focus(node, context.visible_ids);
            let matches_filter = self.matches_filters(
                node,
                context.only,
                context.tag,
                context.ready,
                context.readiness_by_id,
            );
            let has_matching_visible_descendant = !context.ready
                && self.has_matching_visible_descendant(
                    node,
                    context.only,
                    context.tag,
                    context.ready,
                    context.readiness_by_id,
                    context.visible_ids,
                );
            let should_display =
                is_visible_by_focus && (matches_filter || has_matching_visible_descendant);
            let hidden_siblings_at_level = nodes
                .iter()
                .any(|sibling| !self.is_visible_by_focus(sibling, context.visible_ids));

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

                let readiness = readiness_for(node, context.readiness_by_id);
                let node_ready = readiness.ready;
                let has_wip_descendant = has_wip_descendant(node);
                let blocked_by = node
                    .yak
                    .as_ref()
                    .and_then(|y| context.blockers_by_id.get(&y.id).cloned())
                    .unwrap_or_default();

                let yak_context = node.yak.as_ref().and_then(|y| y.context.clone());

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
                    let conn = if is_last_in_full_tree && !hidden_siblings_at_level {
                        "╰─ ".to_string()
                    } else {
                        "├─ ".to_string()
                    };
                    (conn, ancestor_continuations.join(""))
                };

                let child_prefix = prefix.for_child(is_last_in_full_tree, hidden_siblings_at_level);
                let children = self.build_view_tree(&node.children, context, &child_prefix);

                result.push(YakTreeNode {
                    name: node.name.to_string(),
                    full_path: node.full_path.clone(),
                    id,
                    state: state_str,
                    ready: node_ready,
                    readiness,
                    has_wip_descendant,
                    blocked_by,
                    context: yak_context,
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
                let child_prefix = prefix.for_child(is_last_in_full_tree, hidden_siblings_at_level);
                let mut child_nodes = self.build_view_tree(&node.children, context, &child_prefix);
                result.append(&mut child_nodes);
            }
        }
        result
    }

    fn is_visible_by_focus(&self, node: &YakNode, visible_ids: Option<&HashSet<YakId>>) -> bool {
        visible_ids.is_none_or(|ids| {
            node.yak
                .as_ref()
                .map(|y| ids.contains(&y.id))
                .unwrap_or(true)
        })
    }

    fn has_matching_visible_descendant(
        &self,
        node: &YakNode,
        only: Option<&str>,
        tag: Option<&str>,
        ready: bool,
        readiness_by_id: &HashMap<YakId, ReadinessView>,
        visible_ids: Option<&HashSet<YakId>>,
    ) -> bool {
        node.children.iter().any(|child| {
            self.is_visible_by_focus(child, visible_ids)
                && (self.matches_filters(child, only, tag, ready, readiness_by_id)
                    || self.has_matching_visible_descendant(
                        child,
                        only,
                        tag,
                        ready,
                        readiness_by_id,
                        visible_ids,
                    ))
        })
    }

    /// Check if node matches the filters
    fn matches_filters(
        &self,
        node: &YakNode,
        only: Option<&str>,
        tag: Option<&str>,
        ready: bool,
        readiness_by_id: &HashMap<YakId, ReadinessView>,
    ) -> bool {
        let ready_matches = !ready || ready_for(node, readiness_by_id);
        let only_matches = match only {
            Some("done") => node.yak.as_ref().map(|y| y.is_done()).unwrap_or(false),
            Some("not-done") => {
                !node.yak.as_ref().map(|y| y.is_done()).unwrap_or(false) || node.yak.is_none()
            }
            _ => true,
        };
        let tag_matches = tag.is_none_or(|tag| {
            node.yak
                .as_ref()
                .map(|y| y.tags.iter().any(|t| t == tag))
                .unwrap_or(false)
        });
        ready_matches && only_matches && tag_matches
    }
}

impl UseCase for ListYaks {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let format = self.format.as_str();
        let only = self.only.as_deref();
        let tag = self.tag.as_deref();
        let yaks = app.store.list_yaks()?;
        let (readiness_by_id, blockers_by_id): (
            HashMap<YakId, ReadinessView>,
            HashMap<YakId, Vec<YakBlockerView>>,
        ) = app.with_yak_map_result(|map| {
            let readiness = build_readiness_views(map, &yaks);
            let blockers = readiness
                .iter()
                .map(|(id, view)| {
                    let blocked_by = view
                        .reasons
                        .iter()
                        .filter_map(|reason| reason.blocker.clone())
                        .collect::<Vec<_>>();
                    (id.clone(), blocked_by)
                })
                .collect();
            Ok((readiness, blockers))
        })?;
        let visible_ids = app.focused_yak_ids()?;

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

        // Handle ids format early (before tree building) unless readiness needs child state.
        if normalized_format == "ids" && !self.ready {
            for yak in &yaks {
                if visible_ids
                    .as_ref()
                    .is_some_and(|ids| !ids.contains(&yak.id))
                {
                    continue;
                }
                if self.matches_filters(
                    &yak_node_for_filter(yak),
                    only,
                    tag,
                    self.ready,
                    &readiness_by_id,
                ) {
                    app.display
                        .message(&Message::Info(yak.id.as_str().to_string()));
                }
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
        let view_context = ViewBuildContext {
            only,
            tag,
            ready: self.ready,
            readiness_by_id: &readiness_by_id,
            blockers_by_id: &blockers_by_id,
            visible_ids: visible_ids.as_ref(),
        };
        let view_nodes = self.build_view_tree(&tree, &view_context, &TreePrefix::new());

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

fn readiness_for(node: &YakNode, readiness_by_id: &HashMap<YakId, ReadinessView>) -> ReadinessView {
    node.yak
        .as_ref()
        .and_then(|yak| readiness_by_id.get(&yak.id))
        .cloned()
        .unwrap_or(ReadinessView {
            ready: false,
            reasons: vec![],
        })
}

fn ready_for(node: &YakNode, readiness_by_id: &HashMap<YakId, ReadinessView>) -> bool {
    readiness_for(node, readiness_by_id).ready
}

fn has_wip_descendant(node: &YakNode) -> bool {
    node.children.iter().any(|child| {
        child
            .yak
            .as_ref()
            .is_some_and(|yak| yak.state == YakState::Wip)
            || has_wip_descendant(child)
    })
}

/// Recursively build a YakNode and its children from parent_id grouping
fn yak_node_for_filter(yak: &Yak) -> YakNode {
    YakNode {
        name: yak.name.clone(),
        full_path: yak.name.to_string(),
        yak: Some(yak.clone()),
        children: vec![],
    }
}

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
    struct TestWorkspace;

    impl crate::domain::ports::LocalWorkspacePort for TestWorkspace {
        fn is_yaks_gitignored(&self) -> anyhow::Result<bool> {
            Ok(true)
        }

        fn add_yaks_to_gitignore(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn commit_gitignore(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn is_agent_session(&self) -> bool {
            false
        }
    }
    use super::*;
    use crate::adapters::json_display::JsonDisplay;
    use crate::adapters::user_display::{ConsoleDisplay, ConsoleDisplayOptions, TestBuffer};
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::app::set_focus_override;
    use crate::application::{AddYak, Application, SetState};
    use crate::domain::ports::ReadYakStore;
    use crate::infrastructure::EventBus;

    fn make_app<'a>(
        event_store: &'a mut InMemoryEventStore,
        event_bus: &'a mut EventBus,
        storage: &'a InMemoryStorage,
        display: &'a ConsoleDisplay,
        input: &'a InMemoryInput,
        workspace: &'a TestWorkspace,
        auth: &'a InMemoryAuthentication,
    ) -> Application<'a> {
        Application::new(
            event_store,
            event_bus,
            storage,
            display,
            input,
            workspace,
            None,
            auth,
        )
    }

    #[test]
    fn yx_focus_prunes_to_ancestors_focus_and_descendants_with_pruned_markers() {
        set_focus_override(None);
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("Project")).unwrap();
        app.handle(AddYak::new("A").with_parent(Some("Project")))
            .unwrap();
        app.handle(AddYak::new("B").with_parent(Some("A"))).unwrap();
        app.handle(AddYak::new("E").with_parent(Some("B"))).unwrap();
        app.handle(AddYak::new("F").with_parent(Some("B"))).unwrap();
        app.handle(AddYak::new("C").with_parent(Some("A"))).unwrap();
        app.handle(AddYak::new("D").with_parent(Some("Project")))
            .unwrap();
        let focus = ReadYakStore::list_yaks(&storage)
            .unwrap()
            .into_iter()
            .find(|y| y.name.as_str() == "B")
            .unwrap()
            .id;
        buffer.clear();

        set_focus_override(Some(focus.as_str()));
        app.handle(ListYaks::new("pretty", None, None)).unwrap();
        set_focus_override(None);
        let output = buffer.contents();

        assert!(output.contains("Project"));
        assert!(output.contains("A"));
        assert!(output.contains("B"));
        assert!(output.contains("E"));
        assert!(output.contains("F"));
        assert!(!output.contains("C"));
        assert!(!output.contains("D"));
        assert!(
            output.contains("┆  ├─ ○ B"),
            "expected pruned sibling marker before B, got:\n{output}"
        );
        assert!(
            output.contains("┆  ├─ ○ E"),
            "expected pruned sibling marker before B descendants, got:\n{output}"
        );
    }

    #[test]
    fn pretty_tree_shows_todo_parent_with_wip_descendant_as_muted_green_indicator_only() {
        set_focus_override(None);
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let buffer = TestBuffer::new();
        let display = ConsoleDisplay::new(
            Box::new(buffer.clone()),
            ConsoleDisplayOptions {
                color: true,
                width: 60,
            },
        );
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("deploy")).unwrap();
        app.handle(AddYak::new("fix bug").with_parent(Some("deploy")))
            .unwrap();
        app.handle(SetState::new("fix bug", "wip").with_silent(true))
            .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("pretty", None, None)).unwrap();
        let output = buffer.contents();

        assert!(
            output.contains("\x1b[38;5;64m●\x1b[0m deploy"),
            "expected muted green progress-underneath indicator for parent, got:\n{output}"
        );
        assert!(
            !output.contains("\x1b[38;5;64mdeploy"),
            "parent name should not be green, got:\n{output}"
        );
        assert!(
            output.contains("\x1b[32m●\x1b[0m \x1b[1mfix bug\x1b[0m"),
            "expected normal active wip styling for child, got:\n{output}"
        );
    }

    #[test]
    fn filtered_tree_keeps_ancestors_of_matching_grandchild() {
        set_focus_override(None);
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("root")).unwrap();
        app.handle(AddYak::new("branch").with_parent(Some("root")))
            .unwrap();
        app.handle(AddYak::new("leaf").with_parent(Some("branch")))
            .unwrap();
        app.handle(SetState::new("leaf", "done")).unwrap();
        buffer.clear();

        app.handle(ListYaks::new("pretty", Some("done"), None))
            .unwrap();
        let output = buffer.contents();

        assert!(
            output.contains("root"),
            "ancestor of matching grandchild should be visible, got:\n{output}"
        );
        assert!(
            output.contains("branch"),
            "parent of matching grandchild should be visible, got:\n{output}"
        );
        assert!(
            output.contains("leaf"),
            "matching grandchild should be visible, got:\n{output}"
        );
    }

    #[test]
    fn focused_filtered_tree_hides_visible_descendants_that_do_not_match() {
        set_focus_override(None);
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("root")).unwrap();
        app.handle(AddYak::new("focus").with_parent(Some("root")))
            .unwrap();
        app.handle(AddYak::new("child").with_parent(Some("focus")))
            .unwrap();
        let focus = ReadYakStore::list_yaks(&storage)
            .unwrap()
            .into_iter()
            .find(|y| y.name.as_str() == "focus")
            .unwrap()
            .id;
        buffer.clear();

        set_focus_override(Some(focus.as_str()));
        app.handle(ListYaks::new("pretty", Some("done"), None))
            .unwrap();
        set_focus_override(None);
        let output = buffer.contents();

        assert!(
            !output.contains("root") && !output.contains("focus") && !output.contains("child"),
            "focused visible yaks that do not match the filter should stay hidden, got:\n{output}"
        );
    }

    #[test]
    fn focused_ids_format_only_lists_visible_yak_ids() {
        set_focus_override(None);
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("root")).unwrap();
        app.handle(AddYak::new("focus").with_parent(Some("root")))
            .unwrap();
        app.handle(AddYak::new("child").with_parent(Some("focus")))
            .unwrap();
        app.handle(AddYak::new("sibling").with_parent(Some("root")))
            .unwrap();
        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        let id_for = |name: &str| {
            yaks.iter()
                .find(|y| y.name.as_str() == name)
                .unwrap()
                .id
                .as_str()
                .to_string()
        };
        let root_id = id_for("root");
        let focus_id = id_for("focus");
        let child_id = id_for("child");
        let sibling_id = id_for("sibling");
        buffer.clear();

        set_focus_override(Some(&focus_id));
        app.handle(ListYaks::new("ids", None, None)).unwrap();
        set_focus_override(None);
        let output = buffer.contents();
        let ids: Vec<&str> = output.lines().collect();

        assert!(
            ids.contains(&root_id.as_str()),
            "expected root id in {ids:?}"
        );
        assert!(
            ids.contains(&focus_id.as_str()),
            "expected focus id in {ids:?}"
        );
        assert!(
            ids.contains(&child_id.as_str()),
            "expected child id in {ids:?}"
        );
        assert!(
            !ids.contains(&sibling_id.as_str()),
            "sibling outside focus should not be listed in ids output: {ids:?}"
        );
    }

    #[test]
    fn yx_focus_invalid_exact_id_errors() {
        set_focus_override(None);
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, _buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );
        app.handle(AddYak::new("B")).unwrap();

        set_focus_override(Some("B"));
        let err = app
            .handle(ListYaks::new("plain", None, None))
            .unwrap_err()
            .to_string();
        set_focus_override(None);
        assert!(err.contains("YX_FOCUS 'B' does not exactly match a yak id"));
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
        let readiness_by_id = HashMap::new();
        assert!(
            !list.matches_filters(&node, Some("not-done"), None, false, &readiness_by_id),
            "Done yak should be excluded by not-done filter"
        );
    }

    // Lines 183: not-done filter must include not-done yaks
    // Catches both the `!` deletion and `||` to `&&` mutants
    #[test]
    fn not_done_filter_includes_not_done_yaks() {
        let list = ListYaks::new("plain", Some("not-done"), None);
        let node = make_yak_node("pending", "todo");
        let readiness_by_id = HashMap::new();
        assert!(
            list.matches_filters(&node, Some("not-done"), None, false, &readiness_by_id),
            "Not-done yak should be included by not-done filter"
        );
    }

    #[test]
    fn json_list_includes_derived_ready_for_leaf_and_parents() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, _buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;

        {
            let mut app = make_app(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                &workspace,
                &auth,
            );
            app.handle(AddYak::new("leaf")).unwrap();
            app.handle(AddYak::new("blocked parent")).unwrap();
            app.handle(AddYak::new("incomplete child").with_parent(Some("blocked parent")))
                .unwrap();
            app.handle(AddYak::new("ready parent")).unwrap();
            app.handle(AddYak::new("done child").with_parent(Some("ready parent")))
                .unwrap();
            app.handle(SetState::new("done child", "done").with_silent(true))
                .unwrap();
            app.handle(SetState::new("ready parent", "todo").with_silent(true))
                .unwrap();
        }

        let json_buffer = crate::adapters::user_display::TestBuffer::new();
        let json_display = JsonDisplay::with_writer(Box::new(json_buffer.clone()));
        let mut app = Application::new(
            &mut event_store,
            &mut event_bus,
            &storage,
            &json_display,
            &input,
            &workspace,
            None,
            &auth,
        );
        app.handle(ListYaks::new("pretty", None, None)).unwrap();

        let json: serde_json::Value = serde_json::from_str(&json_buffer.contents()).unwrap();
        let nodes = json.as_array().unwrap();
        let leaf = nodes.iter().find(|node| node["name"] == "leaf").unwrap();
        let blocked_parent = nodes
            .iter()
            .find(|node| node["name"] == "blocked parent")
            .unwrap();
        let ready_parent = nodes
            .iter()
            .find(|node| node["name"] == "ready parent")
            .unwrap();

        assert_eq!(leaf["state"], "todo");
        assert_eq!(leaf["ready"], true);
        assert_eq!(blocked_parent["ready"], false);
        assert_eq!(ready_parent["ready"], true);
    }

    #[test]
    fn ids_without_ready_uses_flat_store_order_but_ids_with_ready_uses_tree_order() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("zparent")).unwrap();
        app.handle(AddYak::new("achild").with_parent(Some("zparent")))
            .unwrap();
        let yaks = ReadYakStore::list_yaks(&storage).unwrap();
        let id_for = |name: &str| {
            yaks.iter()
                .find(|yak| yak.name.as_str() == name)
                .unwrap()
                .id
                .as_str()
                .to_string()
        };
        let child_id = id_for("achild");
        let parent_id = id_for("zparent");
        buffer.clear();

        app.handle(ListYaks::new("ids", None, None)).unwrap();
        let ids_without_ready: Vec<String> = buffer.contents().lines().map(String::from).collect();
        assert_eq!(ids_without_ready, vec![child_id.clone(), parent_id]);

        buffer.clear();
        app.handle(ListYaks::new("ids", None, None).with_ready(true))
            .unwrap();
        let ids_with_ready: Vec<String> = buffer.contents().lines().map(String::from).collect();
        assert_eq!(ids_with_ready, vec![child_id]);
    }

    #[test]
    fn ready_filter_includes_only_actionable_todo_yaks() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("leaf")).unwrap();
        app.handle(AddYak::new("unavailable parent")).unwrap();
        app.handle(AddYak::new("ready child").with_parent(Some("unavailable parent")))
            .unwrap();
        app.handle(AddYak::new("wip yak")).unwrap();
        app.handle(SetState::new("wip yak", "wip").with_silent(true))
            .unwrap();
        app.handle(AddYak::new("blocked yak")).unwrap();
        app.handle(crate::application::AddBlocker::manual(
            "blocked yak",
            "waiting",
        ))
        .unwrap();
        app.handle(AddYak::new("done yak")).unwrap();
        app.handle(SetState::new("done yak", "done").with_silent(true))
            .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("plain", None, None).with_ready(true))
            .unwrap();
        let output = buffer.contents();

        assert!(output.contains("leaf"));
        assert!(output.contains("unavailable parent/ready child"));
        assert!(!output.contains("unavailable parent\n"));
        assert!(!output.contains("wip yak"));
        assert!(!output.contains("blocked yak"));
        assert!(!output.contains("done yak"));
    }

    #[test]
    fn not_done_filter_remains_stored_state_not_readiness() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let (display, buffer) = make_test_display();
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
            &auth,
        );

        app.handle(AddYak::new("unavailable parent")).unwrap();
        app.handle(AddYak::new("incomplete child").with_parent(Some("unavailable parent")))
            .unwrap();
        buffer.clear();

        app.handle(ListYaks::new("plain", Some("not-done"), None))
            .unwrap();
        let output = buffer.contents();

        assert!(output.contains("unavailable parent"));
        assert!(output.contains("unavailable parent/incomplete child"));
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
            let workspace = TestWorkspace;
            let mut app = make_app(
                &mut event_store,
                &mut event_bus,
                &storage,
                &display,
                &input,
                &workspace,
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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

    struct TestWorkspace;

    impl crate::domain::ports::LocalWorkspacePort for TestWorkspace {
        fn is_yaks_gitignored(&self) -> anyhow::Result<bool> {
            Ok(true)
        }

        fn add_yaks_to_gitignore(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn commit_gitignore(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn is_agent_session(&self) -> bool {
            false
        }
    }

    fn make_app<'a>(
        event_store: &'a mut InMemoryEventStore,
        event_bus: &'a mut EventBus,
        storage: &'a InMemoryStorage,
        display: &'a ConsoleDisplay,
        input: &'a InMemoryInput,
        workspace: &'a TestWorkspace,
        auth: &'a InMemoryAuthentication,
    ) -> Application<'a> {
        Application::new(
            event_store,
            event_bus,
            storage,
            display,
            input,
            workspace,
            None,
            auth,
        )
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
        let workspace = TestWorkspace;
        let mut app = make_app(
            &mut event_store,
            &mut event_bus,
            &storage,
            &display,
            &input,
            &workspace,
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
