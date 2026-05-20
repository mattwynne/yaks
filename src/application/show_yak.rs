// Use case: Show yak details (yx show)

use anyhow::Result;

use super::{Application, UseCase};
use crate::adapters::views::{YakChildView, YakDetailView};
use crate::application::readiness::build_readiness_views;
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
}

impl UseCase for ShowYak {
    fn execute(&self, app: &mut Application) -> Result<()> {
        let valid_formats = ["pretty"];
        if !valid_formats.contains(&self.format.as_str()) {
            anyhow::bail!(
                "Unknown format '{}'. Valid formats: {}",
                self.format,
                valid_formats.join(", ")
            );
        }

        let id = app.resolve_yak_id(&self.name)?;
        let yak = app.store.get_yak(&id)?;
        let all_yaks = app.store.list_yaks()?;
        let readiness = app.with_yak_map_result(|map| {
            Ok(build_readiness_views(map, &all_yaks).remove(&id).unwrap_or(
                crate::adapters::views::ReadinessView {
                    ready: false,
                    reasons: vec![],
                },
            ))
        })?;
        let visible_ids = app.focused_yak_ids()?;

        // Breadcrumb: walk parent chain to collect ancestors with id, name, state (root-first)
        let mut ancestors = Vec::new();
        let mut current_parent = yak.parent_id.clone();
        while let Some(pid) = current_parent {
            let parent_yak = app.store.get_yak(&pid)?;
            ancestors.push(YakChildView {
                id: parent_yak.id.to_string(),
                name: parent_yak.name.to_string(),
                state: parent_yak.state.to_string(),
            });
            current_parent = parent_yak.parent_id.clone();
        }
        ancestors.reverse();

        // Collect immediate children, sorted by done-state then name
        let children: Vec<YakChildView> = {
            // Find children by scanning all yaks for matching parent_id
            let mut kids: Vec<_> = all_yaks
                .iter()
                .filter(|y| y.parent_id.as_ref() == Some(&yak.id))
                .filter(|y| visible_ids.as_ref().is_none_or(|ids| ids.contains(&y.id)))
                .map(|c| YakChildView {
                    id: c.id.to_string(),
                    name: c.name.to_string(),
                    state: c.state.to_string(),
                })
                .collect();
            kids.sort_by(|a, b| match (a.state == "done", b.state == "done") {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });
            kids
        };

        // Classify custom fields
        let mut short_fields: Vec<(String, String)> = Vec::new();
        let mut long_fields: Vec<(String, String)> = Vec::new();
        let mut field_names: Vec<&str> = yak.fields.keys().map(|k| k.as_str()).collect();
        field_names.sort();
        for name in &field_names {
            let value = yak.fields[*name].as_str().trim();
            if name.ends_with(".md") || value.contains('\n') {
                long_fields.push((title_case(name), value.to_string()));
            } else {
                short_fields.push((title_case(name), value.to_string()));
            }
        }

        // Tags
        let tags: Vec<String> = yak.tags.iter().map(|t| format_tag(t)).collect();

        // Created date
        let created_at = chrono::DateTime::from_timestamp(yak.created_at.as_epoch_secs(), 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Has context?
        let has_context = yak.context.as_ref().is_some_and(|c| !c.trim().is_empty());

        let view = YakDetailView {
            id: id.to_string(),
            breadcrumb: ancestors,
            name: yak.name.to_string(),
            state: yak.state.to_string(),
            readiness,
            created_at,
            created_by: yak.created_by.name.clone(),
            children,
            short_fields,
            long_fields,
            tags,
            context: yak.context.clone(),
            has_context,
        };

        app.display.show_yak(&view);
        Ok(())
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
    fn shows_name_with_state_indicator_and_metadata() {
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
    fn md_fields_always_render_as_long_sections() {
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
        // Single-line .md field — no newlines
        app.handle(WriteField::new("my yak", "comments.md").with_content("A single line note"))
            .unwrap();
        buffer.clear();

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        // .md fields must appear as ruled sections, not in the header box
        assert!(
            output.contains("── Comments.md ─"),
            "Expected ruled section for .md field, got:\n{output}"
        );
        assert!(
            output.contains("  A single line note"),
            "Expected indented .md field content, got:\n{output}"
        );
        assert!(
            !output.contains("Comments.md:"),
            "Should not appear as short header field, got:\n{output}"
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
    fn show_displays_tags_with_at_prefix() {
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        assert!(
            !output.contains("@"),
            "Expected no @ in output when yak has no tags, got:\n{output}"
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

        app.handle(ShowYak::new("my yak", "pretty")).unwrap();
        let output = buffer.contents();
        // "Tags:" would appear if tags were treated as a regular field in the box
        assert!(
            !output.contains("Tags:"),
            "Tags should not appear as a custom field with 'Tags:' label, got:\n{output}"
        );
    }
}

#[cfg(test)]
mod json_tests {
    use crate::adapters::json_display::JsonDisplay;
    use crate::adapters::{
        InMemoryAuthentication, InMemoryEventStore, InMemoryInput, InMemoryStorage,
    };
    use crate::application::{AddYak, Application, ShowYak};
    use crate::infrastructure::EventBus;
    use std::sync::{Arc, Mutex};

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

    /// A shared writer that lets us read back what was written
    #[derive(Clone)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedBuffer {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(Vec::new())))
        }
        fn contents(&self) -> String {
            let buf = self.0.lock().unwrap();
            String::from_utf8(buf.clone()).unwrap()
        }
    }

    impl std::io::Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn json_output_includes_id() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let buffer = SharedBuffer::new();
        let json_display = JsonDisplay::with_writer(Box::new(buffer.clone()));
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
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

        app.handle(AddYak::new("my yak")).unwrap();
        // Clear any output from AddYak
        buffer.0.lock().unwrap().clear();
        app.handle(ShowYak::new("my yak", "pretty")).unwrap();

        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert!(
            json.get("id").is_some(),
            "Expected 'id' field in JSON output, got: {output}"
        );
        let id = json["id"].as_str().unwrap();
        assert!(
            id.starts_with("my-yak-"),
            "Expected id to start with 'my-yak-', got: {id}"
        );
    }

    #[test]
    fn json_output_includes_child_ids_and_structured_ancestors() {
        let mut event_store = InMemoryEventStore::new();
        let mut event_bus = EventBus::new();
        let storage = InMemoryStorage::new();
        event_bus.register(Box::new(storage.clone()));
        let buffer = SharedBuffer::new();
        let json_display = JsonDisplay::with_writer(Box::new(buffer.clone()));
        let input = InMemoryInput::new();
        let auth = InMemoryAuthentication::new();
        let workspace = TestWorkspace;
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

        // Create a nested structure: grandparent > parent > child1, child2
        app.handle(AddYak::new("grandparent")).unwrap();
        app.handle(AddYak::new("parent").with_parent(Some("grandparent")))
            .unwrap();
        app.handle(AddYak::new("child1").with_parent(Some("parent")))
            .unwrap();
        app.handle(AddYak::new("child2").with_parent(Some("parent")))
            .unwrap();

        // Clear any output from AddYak
        buffer.0.lock().unwrap().clear();
        app.handle(ShowYak::new("parent", "pretty")).unwrap();

        let output = buffer.contents();
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();

        // Check children have id, name, and state
        let children = json["children"].as_array().unwrap();
        assert_eq!(
            children.len(),
            2,
            "Expected 2 children, got: {}",
            children.len()
        );

        for child in children {
            assert!(
                child.get("id").is_some(),
                "Expected child to have 'id', got: {child}"
            );
            assert!(
                child.get("name").is_some(),
                "Expected child to have 'name', got: {child}"
            );
            assert!(
                child.get("state").is_some(),
                "Expected child to have 'state', got: {child}"
            );
            let child_id = child["id"].as_str().unwrap();
            let child_name = child["name"].as_str().unwrap();
            assert!(
                child_id.starts_with(&format!("{}-", child_name)),
                "Expected child id to start with '{}-', got: {child_id}",
                child_name
            );
        }

        // Check breadcrumb has structured ancestors with id, name, and state
        let breadcrumb = json["breadcrumb"].as_array().unwrap();
        assert_eq!(
            breadcrumb.len(),
            1,
            "Expected 1 ancestor (grandparent), got: {}",
            breadcrumb.len()
        );

        let ancestor = &breadcrumb[0];
        assert!(
            ancestor.get("id").is_some(),
            "Expected ancestor to have 'id', got: {ancestor}"
        );
        assert_eq!(
            ancestor["name"].as_str().unwrap(),
            "grandparent",
            "Expected ancestor name to be 'grandparent', got: {:?}",
            ancestor["name"]
        );
        assert!(
            ancestor.get("state").is_some(),
            "Expected ancestor to have 'state', got: {ancestor}"
        );
        let ancestor_id = ancestor["id"].as_str().unwrap();
        assert!(
            ancestor_id.starts_with("grandparent-"),
            "Expected ancestor id to start with 'grandparent-', got: {ancestor_id}"
        );
    }
}
