// Use case: Show yak details (yx show)

use anyhow::Result;

use super::{Application, UseCase};
use crate::domain::tag::format_tag;

/// Convert a snake_case field name to Title Case (e.g. "relates_to" → "Relates To")
fn title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct ShowYak {
    name: String,
    format: String,
}

impl ShowYak {
    pub fn new(name: &str, format: &str) -> Self {
        Self {
            name: name.to_string(),
            format: format.to_string(),
        }
    }

    pub fn execute(&self, app: &mut Application) -> Result<()> {
        let valid_formats = ["pretty", "json"];
        if !valid_formats.contains(&self.format.as_str()) {
            anyhow::bail!(
                "Unknown format '{}'. Valid formats: {}",
                self.format,
                valid_formats.join(", ")
            );
        }

        let id = app.store.fuzzy_find_yak_id(&self.name)?;
        let yak = app.store.get_yak(&id)?;

        if self.format == "json" {
            let children: Vec<serde_json::Value> = yak
                .children
                .iter()
                .filter_map(|cid| app.store.get_yak(cid).ok())
                .map(|c| {
                    serde_json::json!({
                        "id": c.id.as_str(),
                        "name": c.name.as_str(),
                        "state": c.state,
                    })
                })
                .collect();

            let fields: serde_json::Map<String, serde_json::Value> = yak
                .fields
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();

            let tags: Vec<&str> = yak.tags.iter().map(|s| s.as_str()).collect();

            let json = serde_json::json!({
                "id": id.as_str(),
                "name": yak.name.as_str(),
                "state": yak.state,
                "parent_id": yak.parent_id.as_ref().map(|p| p.as_str()),
                "context": yak.context,
                "fields": fields,
                "children": children,
                "tags": tags,
            });

            app.display.info(&serde_json::to_string_pretty(&json)?);
            return Ok(());
        }

        // Breadcrumb: walk parent chain to collect ancestor names (root-first)
        let mut ancestors = Vec::new();
        let mut current_parent = yak.parent_id.clone();
        while let Some(pid) = current_parent {
            let parent_yak = app.store.get_yak(&pid)?;
            ancestors.push(parent_yak.name.clone());
            current_parent = parent_yak.parent_id.clone();
        }
        ancestors.reverse();

        // Collect immediate children for the header box
        let box_children: Vec<_> = {
            let mut kids: Vec<_> = yak
                .children
                .iter()
                .filter_map(|id| app.store.get_yak(id).ok())
                .map(|c| (c.name.clone(), c.state.clone()))
                .collect();
            kids.sort_by(|a, b| match (a.1 == "done", b.1 == "done") {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(&b.0),
            });
            kids
        };

        // Classify custom fields: no newlines = short (in box), newlines = long (ruled section)
        let mut short_fields: Vec<(String, String)> = Vec::new();
        let mut long_fields: Vec<(&str, &str)> = Vec::new();
        let mut field_names: Vec<&str> = yak.fields.keys().map(|k| k.as_str()).collect();
        field_names.sort();
        for name in &field_names {
            let value = yak.fields[*name].as_str().trim();
            if value.contains('\n') {
                long_fields.push((name, value));
            } else {
                short_fields.push((title_case(name), value.to_string()));
            }
        }

        // Extract tags for display
        let tags: Vec<String> = yak.tags.iter().map(|t| format_tag(t)).collect();

        // Header box with breadcrumb, name, state, date, author, children, short fields, and tags
        app.display.display_header_box(
            &ancestors,
            &yak.name,
            &yak.state,
            &yak.created_at,
            &yak.created_by,
            &box_children,
            &short_fields,
            &tags,
        );

        // Context body
        let has_context = yak.context.as_ref().is_some_and(|c| !c.trim().is_empty());
        if has_context {
            app.display.info("");
            app.display.display_context(yak.context.as_ref().unwrap());
        } else {
            app.display.info("");
            app.display.display_hint(&format!(
                "This yak has no context yet. Add some with:\n\n  echo \"Here's the problem...\" | yx context {}",
                yak.name
            ));
        }

        // Long fields in ruled sections
        if !long_fields.is_empty() {
            for (name, value) in long_fields.iter() {
                app.display.info("");
                app.display.display_section_rule(&title_case(name));
                let indented: String = value
                    .lines()
                    .map(|l| format!("  {l}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                app.display.info(&indented);
            }
        }

        app.display.info("");
        app.display.display_closing_rule();

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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
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

        app.handle(ShowYak::new("root yak", "pretty")).unwrap();
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

        app.handle(ShowYak::new("child", "pretty")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        // First line: top border of box
        assert!(
            lines[0].starts_with('┌'),
            "Expected box top border, got: {:?}",
            lines[0]
        );
        // Second line: breadcrumb inside the box
        assert!(
            lines[1].contains("grandparent > parent >"),
            "Expected breadcrumb inside box, got: {:?}",
            lines[1]
        );
        assert!(
            lines[1].starts_with('│'),
            "Breadcrumb should be inside box border, got: {:?}",
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("This yak has no context yet"),
            "Expected hint message when no context, got:\n{output}"
        );
        assert!(
            output.contains("yx context my yak"),
            "Expected hint with yak name, got:\n{output}"
        );
    }

    #[test]
    fn children_appear_inside_header_box() {
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

        app.handle(ShowYak::new("parent", "pretty")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        // Children should be inside the box (between ┌ and └)
        let top = lines.iter().position(|l| l.starts_with('┌')).unwrap();
        let bottom = lines.iter().position(|l| l.starts_with('└')).unwrap();
        let alpha_pos = lines.iter().position(|l| l.contains("alpha")).unwrap();
        let beta_pos = lines.iter().position(|l| l.contains("beta")).unwrap();
        assert!(
            alpha_pos > top && alpha_pos < bottom,
            "alpha should be inside box"
        );
        assert!(
            beta_pos > top && beta_pos < bottom,
            "beta should be inside box"
        );

        // Tree connectors
        let alpha_line = lines[alpha_pos];
        let beta_line = lines[beta_pos];
        assert!(
            alpha_line.contains("├─"),
            "Non-last child should have ├─, got: {:?}",
            alpha_line
        );
        assert!(
            beta_line.contains("╰─"),
            "Last child should have ╰─, got: {:?}",
            beta_line
        );
    }

    #[test]
    fn no_children_in_box_when_none() {
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

        app.handle(ShowYak::new("lonely", "pretty")).unwrap();
        let output = buffer.contents();
        // No children in box — box should only have 3 lines (┌, │, └)
        let lines: Vec<&str> = output.lines().collect();
        let top = lines.iter().position(|l| l.starts_with('┌')).unwrap();
        let bottom = lines.iter().position(|l| l.starts_with('└')).unwrap();
        assert_eq!(
            bottom - top,
            2,
            "Box should have 3 lines (no children), got: {:?}",
            &lines[top..=bottom]
        );
    }

    #[test]
    fn single_line_fields_appear_inside_box() {
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
        app.handle(WriteField::new("my yak", "relates_to").with_content("foo-bar"))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();

        // Fields should be inside the box (between ┌ and └)
        let top = lines.iter().position(|l| l.starts_with('┌')).unwrap();
        let bottom = lines.iter().position(|l| l.starts_with('└')).unwrap();

        // Divider bar between header and fields
        let divider = lines.iter().position(|l| l.starts_with('├'));
        assert!(divider.is_some(), "Expected divider bar, got:\n{output}");
        let divider = divider.unwrap();
        assert!(
            divider > top && divider < bottom,
            "Divider should be inside box"
        );

        // Title Case field names
        let priority_line = lines.iter().find(|l| l.contains("Priority:"));
        assert!(
            priority_line.is_some(),
            "Expected 'Priority:' (Title Case), got:\n{output}"
        );
        assert!(
            priority_line.unwrap().contains("high"),
            "Expected 'high' value, got: {:?}",
            priority_line
        );

        let relates_line = lines.iter().find(|l| l.contains("Relates To:"));
        assert!(
            relates_line.is_some(),
            "Expected 'Relates To:' (Title Case), got:\n{output}"
        );
        assert!(
            relates_line.unwrap().contains("foo-bar"),
            "Expected 'foo-bar' value, got: {:?}",
            relates_line
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        // Should have a ruled header with field name
        assert!(
            output.contains("── Notes ─"),
            "Expected ruled header for 'Notes', got:\n{output}"
        );
        assert!(
            output.contains("  Line one\n  Line two\n  Line three"),
            "Expected indented long field content, got:\n{output}"
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        assert!(
            !output.contains("── "),
            "Expected no ruled field sections, got:\n{output}"
        );
    }

    #[test]
    fn long_single_line_value_goes_in_box() {
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
        let long_value = "a".repeat(60);
        app.handle(WriteField::new("my yak", "description").with_content(&long_value))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        // Single-line field goes in box, not in ruled section
        assert!(
            output.contains("Description:"),
            "Expected field in box, got:\n{output}"
        );
        assert!(
            !output.contains("── description"),
            "Should not have ruled section for single-line field, got:\n{output}"
        );
    }

    #[test]
    fn json_output_includes_all_fields() {
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

        app.handle(
            AddYak::new("parent yak")
                .with_context(Some("some notes"))
                .with_state(Some("wip"))
                .with_field("plan", "step 1"),
        )
        .unwrap();
        app.handle(AddYak::new("child").with_parent(Some("parent yak")))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("parent yak", "json")).unwrap();
        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput:\n{output}"));

        assert_eq!(json["name"], "parent yak");
        assert_eq!(json["state"], "wip");
        assert_eq!(json["context"], "some notes");
        assert_eq!(json["parent_id"], serde_json::Value::Null);
        assert_eq!(json["fields"]["plan"], "step 1");

        let children = json["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["name"], "child");
        assert_eq!(children[0]["state"], "todo");
        assert!(children[0]["id"].as_str().unwrap().starts_with("child-"));
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

        let result = app.handle(ShowYak::new("nonexistent", "pretty"));
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod tag_tests {
    use crate::adapters::user_display::ConsoleDisplay;
    use crate::adapters::{
        make_test_display, InMemoryAuthentication, InMemoryEventStore, InMemoryInput,
        InMemoryStorage,
    };
    use crate::application::{AddTag, AddYak, Application, ShowYak};
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
    fn show_displays_tags_with_at_prefix() {
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        assert!(
            output.contains("@v1.0"),
            "Expected @v1.0 in output, got:\n{output}"
        );
        assert!(
            output.contains("@needs-review"),
            "Expected @needs-review in output, got:\n{output}"
        );
    }

    #[test]
    fn show_without_tags_has_no_at_symbol() {
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        assert!(
            !output.contains("@"),
            "Expected no @ in output when yak has no tags, got:\n{output}"
        );
    }

    #[test]
    fn json_output_includes_tags_array() {
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

        app.handle(ShowYak::new("my yak", "json")).unwrap();
        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput:\n{output}"));

        let tags = json["tags"]
            .as_array()
            .expect("Expected tags array in JSON");
        assert!(
            tags.contains(&serde_json::json!("v1.0")),
            "Expected v1.0 in tags: {:?}",
            tags
        );
        assert!(
            tags.contains(&serde_json::json!("needs-review")),
            "Expected needs-review in tags: {:?}",
            tags
        );
    }

    #[test]
    fn json_output_has_empty_tags_when_no_tags() {
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

        app.handle(ShowYak::new("my yak", "json")).unwrap();
        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output)
            .unwrap_or_else(|e| panic!("Invalid JSON: {e}\nOutput:\n{output}"));

        let tags = json["tags"]
            .as_array()
            .expect("Expected tags array in JSON");
        assert!(
            tags.is_empty(),
            "Expected empty tags array, got: {:?}",
            tags
        );
    }

    #[test]
    fn tags_field_not_shown_in_custom_fields_section() {
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        // "Tags:" would appear if tags were treated as a regular field in the box
        assert!(
            !output.contains("Tags:"),
            "Tags should not appear as a custom field with 'Tags:' label, got:\n{output}"
        );
    }
}
