// Use case: Show yak details (yx show)

use anyhow::Result;

use crate::domain::slug::YakId;

use super::{Application, UseCase};

pub struct ShowYak {
    name: String,
}

impl ShowYak {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let id = app.store.fuzzy_find_yak_id(&self.name)?;
        let yak = app.store.get_yak(&id)?;

        // Breadcrumb: walk parent chain to collect ancestor names (root-first)
        let mut ancestors = Vec::new();
        let mut current_parent = yak.parent_id.clone();
        while let Some(pid) = current_parent {
            let parent_yak = app.store.get_yak(&pid)?;
            ancestors.push(parent_yak.name.clone());
            current_parent = parent_yak.parent_id.clone();
        }
        ancestors.reverse();
        app.display.display_breadcrumb(&ancestors);

        // Header box with name, state, date, author
        app.display
            .display_header_box(&yak.name, &yak.state, &yak.created_at, &yak.created_by);

        // Classify custom fields into short and long
        let mut short_fields: Vec<(&str, &str)> = Vec::new();
        let mut long_fields: Vec<(&str, &str)> = Vec::new();
        let mut field_names: Vec<&str> = yak.fields.keys().map(|k| k.as_str()).collect();
        field_names.sort();
        for name in &field_names {
            let value = yak.fields[*name].as_str();
            if value.contains('\n') || value.len() >= 60 {
                long_fields.push((name, value));
            } else {
                short_fields.push((name, value));
            }
        }

        // Short fields inline after metadata
        if !short_fields.is_empty() {
            let line = short_fields
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join(" · ");
            app.display.info(&line);
        }

        // Context body (if present and non-empty)
        if let Some(ref context) = yak.context {
            if !context.trim().is_empty() {
                app.display.info("");
                app.display.display_context(context);
            }
        }

        // Child subtree
        if !yak.children.is_empty() {
            let rule_width: usize = 60;
            app.display.info("");
            let header = "── children ";
            let padding = rule_width.saturating_sub(header.len());
            app.display
                .info(&format!("{header}{}", "─".repeat(padding)));
            app.display.info("");
            app.display.info("  ⋮");
            Self::display_subtree(app, &yak.children, "  ")?;
        }

        // Long fields in ruled sections
        if !long_fields.is_empty() {
            let rule_width: usize = 60;
            for (i, (name, value)) in long_fields.iter().enumerate() {
                app.display.info("");
                // Header rule: ── name ────...
                let header = format!("── {name} ");
                let padding = rule_width.saturating_sub(header.len());
                let header_rule = format!("{header}{}", "─".repeat(padding));
                app.display.info(&header_rule);
                app.display.info(value);
                // Closing rule only after the last long field
                if i == long_fields.len() - 1 {
                    app.display.info(&"─".repeat(rule_width));
                }
            }
        }

        Ok(())
    }

    fn display_subtree(
        app: &mut Application,
        child_ids: &[YakId],
        prefix: &str,
    ) -> Result<()> {
        // Fetch and sort children (done first, then alphabetical)
        let mut children: Vec<_> = child_ids
            .iter()
            .filter_map(|id| app.store.get_yak(id).ok())
            .collect();
        children.sort_by(|a, b| {
            match (a.state == "done", b.state == "done") {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let connector = if is_last { "╰─ " } else { "├─ " };
            let node_prefix = format!("{prefix}{connector}");
            app.display
                .display_yak_pretty(&node_prefix, &child.name, &child.state);

            if !child.children.is_empty() {
                let continuation = if is_last { "   " } else { "│  " };
                let child_prefix = format!("{prefix}{continuation}");
                Self::display_subtree(app, &child.children, &child_prefix)?;
            }
        }
        Ok(())
    }
}

impl UseCase for ShowYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        Self::execute(self, app)
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
    use crate::application::{AddYak, EditContext, WriteField};
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
    fn shows_name_with_state_indicator_and_metadata() {
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

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        // Header box
        assert!(
            lines[0].starts_with('┌'),
            "Expected top border, got: {:?}",
            lines[0]
        );
        assert!(
            lines[1].contains("○ my yak"),
            "Expected name in box, got: {:?}",
            lines[1]
        );
        assert!(
            lines[1].contains("todo"),
            "Expected state in box, got: {:?}",
            lines[1]
        );
        assert!(
            lines[2].starts_with('└'),
            "Expected bottom border, got: {:?}",
            lines[2]
        );
    }

    #[test]
    fn root_yak_has_no_breadcrumb_line() {
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

        app.handle(AddYak::new("root yak")).unwrap();
        buffer.clear();

        app.handle(ShowYak::new("root yak")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        // First line should be top border, not a breadcrumb
        assert!(
            lines[0].starts_with('┌'),
            "Expected box top border as first line, got: {:?}",
            lines[0]
        );
    }

    #[test]
    fn nested_yak_shows_breadcrumb_path() {
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

        app.handle(AddYak::new("grandparent")).unwrap();
        app.handle(AddYak::new("parent").with_parent(Some("grandparent")))
            .unwrap();
        app.handle(AddYak::new("child").with_parent(Some("parent")))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("child")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        // First line: breadcrumb
        assert_eq!(
            lines[0], "grandparent > parent > ",
            "Expected breadcrumb path, got: {:?}",
            lines[0]
        );
        // Second line: top border of box
        assert!(
            lines[1].starts_with('┌'),
            "Expected box top border on second line, got: {:?}",
            lines[1]
        );
    }

    #[test]
    fn shows_context_below_metadata() {
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
        input.set_content(Some("Here is some context about this yak.".to_string()));
        app.handle(EditContext::new("my yak")).unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("Here is some context about this yak."),
            "Expected context in output, got: {output}"
        );
        // Context should appear after the header box
        let box_pos = output.find('└').unwrap();
        let context_pos = output.find("Here is some context").unwrap();
        assert!(
            context_pos > box_pos,
            "Context should appear after header box"
        );
    }

    #[test]
    fn no_context_section_when_context_is_empty() {
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

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        // Should be just: name, blank, metadata (3 lines)
        assert_eq!(
            lines.len(),
            3,
            "Expected 3 lines (no context section), got {} lines: {:?}",
            lines.len(),
            lines
        );
    }

    #[test]
    fn shows_child_subtree_below_metadata() {
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
        app.handle(AddYak::new("alpha").with_parent(Some("parent")))
            .unwrap();
        app.handle(AddYak::new("beta").with_parent(Some("parent")))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("parent")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        // Should have a ruled header for children
        assert!(
            output.contains("── children ─"),
            "Expected ruled header for children section, got:\n{output}"
        );

        // Should contain children with tree connectors
        let alpha_line = lines.iter().find(|l| l.contains("alpha"));
        let beta_line = lines.iter().find(|l| l.contains("beta"));
        assert!(
            alpha_line.is_some(),
            "Expected child 'alpha' in output: {:?}",
            lines
        );
        assert!(
            beta_line.is_some(),
            "Expected child 'beta' in output: {:?}",
            lines
        );
        assert!(
            alpha_line.unwrap().starts_with("  ├─"),
            "Non-last child should have indented ├─ connector, got: {:?}",
            alpha_line
        );
        assert!(
            beta_line.unwrap().starts_with("  ╰─"),
            "Last child should have indented ╰─ connector, got: {:?}",
            beta_line
        );
    }

    #[test]
    fn no_subtree_when_no_children() {
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

        app.handle(AddYak::new("lonely")).unwrap();
        buffer.clear();

        app.handle(ShowYak::new("lonely")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        // Should be just: name, blank, metadata (3 lines)
        assert_eq!(
            lines.len(),
            3,
            "Expected 3 lines (no subtree), got {} lines: {:?}",
            lines.len(),
            lines
        );
    }

    #[test]
    fn shows_nested_grandchildren_in_subtree() {
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

        app.handle(AddYak::new("root")).unwrap();
        app.handle(AddYak::new("child").with_parent(Some("root")))
            .unwrap();
        app.handle(AddYak::new("grandchild").with_parent(Some("child")))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("root")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        let grandchild_line = lines.iter().find(|l| l.contains("grandchild"));
        assert!(
            grandchild_line.is_some(),
            "Expected grandchild in output: {:?}",
            lines
        );
        // Grandchild under last child should have "   ╰─" prefix
        assert!(
            grandchild_line.unwrap().contains("╰─"),
            "Grandchild should have tree connector, got: {:?}",
            grandchild_line
        );
    }

    #[test]
    fn short_fields_appear_after_metadata() {
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
        app.handle(WriteField::new("my yak", "priority").with_content("high"))
            .unwrap();
        app.handle(WriteField::new("my yak", "team").with_content("platform"))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        // Short fields appear after the box bottom border
        let box_bottom = lines.iter().position(|l| l.starts_with('└')).unwrap();
        let fields_line = &lines[box_bottom + 1];
        assert!(
            fields_line.contains("priority: high"),
            "Expected 'priority: high' in short fields line, got: {:?}",
            fields_line
        );
        assert!(
            fields_line.contains("team: platform"),
            "Expected 'team: platform' in short fields line, got: {:?}",
            fields_line
        );
        assert!(
            fields_line.contains(" · "),
            "Expected fields joined with ' · ', got: {:?}",
            fields_line
        );
    }

    #[test]
    fn long_fields_appear_in_ruled_sections_at_bottom() {
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
        let long_content = "Line one\nLine two\nLine three";
        app.handle(WriteField::new("my yak", "notes").with_content(long_content))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        // Should have a ruled header with field name
        assert!(
            output.contains("── notes ─"),
            "Expected ruled header for 'notes', got:\n{output}"
        );
        assert!(
            output.contains("Line one\nLine two\nLine three"),
            "Expected long field content, got:\n{output}"
        );
        // Last field gets a closing rule
        assert!(
            output.contains("──────────"),
            "Expected closing rule, got:\n{output}"
        );
    }

    #[test]
    fn no_field_sections_when_no_custom_fields() {
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

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        assert!(
            !output.contains("── "),
            "Expected no ruled field sections, got:\n{output}"
        );
    }

    #[test]
    fn long_value_on_single_line_goes_to_ruled_section() {
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
        let long_value = "a".repeat(60); // exactly 60 chars = long
        app.handle(WriteField::new("my yak", "description").with_content(&long_value))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak")).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("── description ─"),
            "Expected ruled header for long single-line field, got:\n{output}"
        );
    }

    #[test]
    fn error_when_yak_not_found() {
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

        let result = app.handle(ShowYak::new("nonexistent"));
        assert!(result.is_err());
    }
}
