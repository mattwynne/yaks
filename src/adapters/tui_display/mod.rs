// TUI adapter - implementation using ratatui for rich terminal output

use crate::adapters::user_display::ConsoleDisplay;
use crate::adapters::views::{LogEntryView, Message, YakDetailView, YakTreeView};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Terminal, TerminalOptions, Viewport};
use std::io;

pub struct TuiDisplay {
    width: usize,
    /// Fallback for methods not yet converted to ratatui.
    fallback: ConsoleDisplay,
}

impl TuiDisplay {
    pub fn new(width: usize) -> Self {
        use crate::adapters::user_display::ConsoleDisplayOptions;
        Self {
            width,
            fallback: ConsoleDisplay::new(
                Box::new(io::stdout()),
                ConsoleDisplayOptions { color: true, width },
            ),
        }
    }

    pub fn stdout() -> Self {
        let width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);
        Self {
            width,
            fallback: ConsoleDisplay::stdout(),
        }
    }

    pub fn with_writer(writer: Box<dyn std::io::Write + Send>) -> Self {
        use crate::adapters::user_display::ConsoleDisplayOptions;
        let width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);
        Self {
            width,
            fallback: ConsoleDisplay::new(writer, ConsoleDisplayOptions { color: true, width }),
        }
    }

    /// Render the header box from a YakDetailView into a ratatui Buffer
    fn render_show_header_box(
        &self,
        view: &YakDetailView,
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
    ) {
        fn indicator_for(state: &str) -> &'static str {
            match state {
                "wip" => "●",
                "blocked" => "⏸",
                "done" => "✓",
                _ => "○",
            }
        }

        fn state_color(state: &str) -> Color {
            match state {
                "wip" => Color::Green,
                "blocked" => Color::Yellow,
                "done" => Color::DarkGray,
                _ => Color::Reset,
            }
        }

        let state = view.state.as_str();
        let indicator = indicator_for(state);
        let mut lines: Vec<Line> = Vec::new();

        // Breadcrumb
        if !view.breadcrumb.is_empty() {
            let path = view
                .breadcrumb
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(" > ");
            lines.push(Line::from(Span::styled(
                format!(" {path} >"),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }

        // Header line
        let mut header_spans: Vec<Span> = Vec::new();
        header_spans.push(Span::raw(" "));
        header_spans.push(Span::styled(
            indicator,
            Style::default().fg(state_color(state)),
        ));
        header_spans.push(Span::raw(" "));

        match state {
            "done" => header_spans.push(Span::styled(
                &view.name,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT),
            )),
            _ => header_spans.push(Span::styled(
                &view.name,
                Style::default().add_modifier(Modifier::BOLD),
            )),
        }

        header_spans.push(Span::styled(
            format!(" · {} · {} · {}", state, view.created_at, view.created_by),
            Style::default().fg(Color::DarkGray),
        ));

        if !view.tags.is_empty() {
            header_spans.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            header_spans.push(Span::styled(
                view.tags.join(" "),
                Style::default().fg(Color::Rgb(95, 135, 175)),
            ));
        }

        lines.push(Line::from(header_spans));

        // Children
        for (i, child) in view.children.iter().enumerate() {
            let connector = if i == view.children.len() - 1 {
                "╰─"
            } else {
                "├─"
            };
            let ci = indicator_for(&child.state);
            let mut spans: Vec<Span> = Vec::new();
            spans.push(Span::raw(format!(" {connector} ")));
            spans.push(Span::styled(
                ci,
                Style::default().fg(state_color(&child.state)),
            ));
            spans.push(Span::raw(" "));
            match child.state.as_str() {
                "done" => spans.push(Span::styled(
                    &child.name,
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::CROSSED_OUT),
                )),
                "wip" => spans.push(Span::styled(
                    &child.name,
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                _ => spans.push(Span::raw(&child.name)),
            }
            lines.push(Line::from(spans));
        }

        // Fields
        if !view.short_fields.is_empty() {
            let divider_width = area.width.saturating_sub(2) as usize;
            lines.push(Line::from(Span::styled(
                "─".repeat(divider_width),
                Style::default().add_modifier(Modifier::DIM),
            )));

            let max_label = view
                .short_fields
                .iter()
                .map(|(k, _)| k.chars().count())
                .max()
                .unwrap_or(0);
            for (k, v) in &view.short_fields {
                let pad = max_label - k.chars().count();
                lines.push(Line::from(format!(" {}{}: {}", " ".repeat(pad), k, v)));
            }
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().add_modifier(Modifier::DIM));

        let paragraph = Paragraph::new(lines).block(block);
        ratatui::widgets::Widget::render(paragraph, area, buf);
    }
}

impl crate::domain::ports::DisplayPort for TuiDisplay {
    fn width(&self) -> usize {
        self.width
    }

    fn show_yak(&self, view: &YakDetailView) {
        use ratatui::backend::CrosstermBackend;

        // Calculate header box height from view model
        let has_breadcrumb = !view.breadcrumb.is_empty();
        let mut height: u16 = 3; // top + header + bottom
        if has_breadcrumb {
            height += 1;
        }
        height += view.children.len() as u16;
        if !view.short_fields.is_empty() {
            height += 1 + view.short_fields.len() as u16;
        }

        // Render header box with ratatui
        let backend = CrosstermBackend::new(io::stdout());
        if let Ok(mut terminal) = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        ) {
            let _ = terminal.draw(|frame| {
                let area = frame.area();
                self.render_show_header_box(view, area, frame.buffer_mut());
            });
            let _ = terminal.show_cursor();
        }
        println!();

        // Context (or hint) — delegate to fallback for now
        // (termimad markdown rendering is complex to replicate in ratatui)
        if view.has_context {
            self.fallback.info("");
            self.fallback
                .display_context(view.context.as_ref().unwrap());
        } else {
            self.fallback.info("");
            self.fallback.display_hint(&format!(
                "This yak has no context yet. Add some with:\n\n  echo \"Here's the problem...\" | yx context {}",
                view.name
            ));
        }

        // Long fields
        for (name, value) in &view.long_fields {
            self.fallback.info("");
            self.fallback.display_section_rule(name);
            let indented: String = value
                .lines()
                .map(|l| format!("  {l}"))
                .collect::<Vec<_>>()
                .join("\n");
            self.fallback.info(&indented);
        }

        self.fallback.info("");
        self.fallback.display_closing_rule();
    }

    fn show_list(&self, view: &YakTreeView) {
        self.fallback.show_list(view);
    }

    fn show_log(&self, entries: &[LogEntryView]) {
        self.fallback.show_log(entries);
    }

    fn message(&self, msg: &Message) {
        self.fallback.message(msg);
    }

    fn show_help(&self, help_text: &str) {
        self.fallback.show_help(help_text);
    }

    fn start_progress(&self, message: &str) -> Box<dyn crate::domain::ports::ProgressHandle> {
        let handle = crate::adapters::spinner::SpinnerHandle::start(
            message.to_string(),
            Box::new(io::stdout()),
        );
        Box::new(handle)
    }
}

/// Extract text content from a ratatui Buffer, trimming trailing
/// whitespace from each line.
#[cfg(test)]
fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let area = buffer.area;
    let mut result = String::new();
    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            line.push_str(buffer[(x, y)].symbol());
        }
        result.push_str(line.trim_end());
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::views::YakChildView;
    use ratatui::backend::TestBackend;

    /// Helper to build a YakDetailView for tests with sensible defaults
    #[allow(clippy::too_many_arguments)]
    fn make_detail_view(
        id: &str,
        breadcrumb: Vec<YakChildView>,
        name: &str,
        state: &str,
        created_at: &str,
        created_by: &str,
        children: Vec<YakChildView>,
        short_fields: Vec<(String, String)>,
        tags: Vec<String>,
    ) -> YakDetailView {
        YakDetailView {
            id: id.to_string(),
            breadcrumb,
            name: name.to_string(),
            state: state.to_string(),
            created_at: created_at.to_string(),
            created_by: created_by.to_string(),
            children,
            short_fields,
            long_fields: vec![],
            tags,
            context: None,
            has_context: false,
        }
    }

    /// Create a TestBackend terminal for the given YakDetailView,
    /// then render it using render_show_header_box. Returns the
    /// terminal so tests can inspect the buffer.
    fn draw_test_show_header_box(width: u16, view: &YakDetailView) -> Terminal<TestBackend> {
        // Calculate height from view
        let mut height: u16 = 3; // top + header + bottom
        if !view.breadcrumb.is_empty() {
            height += 1;
        }
        height += view.children.len() as u16;
        if !view.short_fields.is_empty() {
            height += 1 + view.short_fields.len() as u16;
        }

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, width, height)),
            },
        )
        .unwrap();
        let display = TuiDisplay::new(width as usize);
        let _ = terminal.draw(|frame| {
            let area = frame.area();
            display.render_show_header_box(view, area, frame.buffer_mut());
        });
        let _ = terminal.show_cursor();
        terminal
    }

    #[test]
    fn header_box_renders_box_drawing_borders() {
        let view = make_detail_view(
            "my-yak-abc1",
            vec![],
            "my yak",
            "wip",
            "2025-02-19",
            "Matt Wynne",
            vec![],
            vec![],
            vec![],
        );
        let terminal = draw_test_show_header_box(60, &view);

        let output = buffer_to_string(terminal.backend().buffer());
        let lines: Vec<&str> = output.lines().collect();

        assert!(lines[0].starts_with('┌'), "got: {:?}", lines[0]);
        assert!(lines[0].ends_with('┐'), "got: {:?}", lines[0]);
        assert!(lines[1].contains("● my yak"), "got: {:?}", lines[1]);
        assert!(lines[1].contains("wip"), "got: {:?}", lines[1]);
        assert!(lines[1].contains("2025-02-19"), "got: {:?}", lines[1]);
        assert!(lines[1].contains("Matt Wynne"), "got: {:?}", lines[1]);
        assert!(lines[2].starts_with('└'), "got: {:?}", lines[2]);
        assert!(lines[2].ends_with('┘'), "got: {:?}", lines[2]);
    }

    #[test]
    fn header_box_shows_breadcrumb_for_ancestors() {
        let breadcrumb = vec![YakChildView {
            id: "parent-xyz9".to_string(),
            name: "parent".to_string(),
            state: "wip".to_string(),
        }];
        let view = make_detail_view(
            "child-yak-def2",
            breadcrumb,
            "child yak",
            "todo",
            "2025-02-19",
            "Matt Wynne",
            vec![],
            vec![],
            vec![],
        );
        let terminal = draw_test_show_header_box(60, &view);

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("parent >"), "got:\n{output}");
        assert!(output.contains("child yak"), "got:\n{output}");
    }

    #[test]
    fn header_box_shows_children() {
        let children = vec![
            YakChildView {
                id: "child-one-ghi3".to_string(),
                name: "child one".to_string(),
                state: "todo".to_string(),
            },
            YakChildView {
                id: "child-two-jkl4".to_string(),
                name: "child two".to_string(),
                state: "wip".to_string(),
            },
        ];
        let view = make_detail_view(
            "parent-yak-mno5",
            vec![],
            "parent yak",
            "todo",
            "2025-02-19",
            "Matt Wynne",
            children,
            vec![],
            vec![],
        );
        let terminal = draw_test_show_header_box(60, &view);

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("├─"), "got:\n{output}");
        assert!(output.contains("╰─"), "got:\n{output}");
        assert!(output.contains("child one"), "got:\n{output}");
        assert!(output.contains("child two"), "got:\n{output}");
    }

    #[test]
    fn header_box_shows_fields() {
        let short_fields = vec![
            ("worktree".to_string(), "/tmp/wt".to_string()),
            ("branch".to_string(), "feat-x".to_string()),
        ];
        let view = make_detail_view(
            "my-yak-pqr6",
            vec![],
            "my yak",
            "wip",
            "2025-02-19",
            "Matt Wynne",
            vec![],
            short_fields,
            vec![],
        );
        let terminal = draw_test_show_header_box(60, &view);

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("worktree: /tmp/wt"), "got:\n{output}");
        assert!(output.contains("branch: feat-x"), "got:\n{output}");
    }

    #[test]
    fn header_box_shows_tags() {
        let tags = vec!["@bug".to_string(), "@urgent".to_string()];
        let view = make_detail_view(
            "tagged-yak-stu7",
            vec![],
            "tagged yak",
            "todo",
            "2025-02-19",
            "Matt Wynne",
            vec![],
            vec![],
            tags,
        );
        let terminal = draw_test_show_header_box(80, &view);

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("@bug"), "got:\n{output}");
        assert!(output.contains("@urgent"), "got:\n{output}");
    }

    #[test]
    fn header_box_done_state_uses_filled_indicator() {
        let view = make_detail_view(
            "done-yak-vwx8",
            vec![],
            "done yak",
            "done",
            "2025-02-19",
            "Matt Wynne",
            vec![],
            vec![],
            vec![],
        );
        let terminal = draw_test_show_header_box(60, &view);

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("✓ done yak"), "got:\n{output}");
    }

    #[test]
    fn header_box_todo_state_uses_open_indicator() {
        let view = make_detail_view(
            "todo-yak-yza9",
            vec![],
            "todo yak",
            "todo",
            "2025-02-19",
            "Matt Wynne",
            vec![],
            vec![],
            vec![],
        );
        let terminal = draw_test_show_header_box(60, &view);

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("○ todo yak"), "got:\n{output}");
    }

    #[test]
    fn header_box_bold_style_on_wip_name() {
        let view = make_detail_view(
            "wip-yak-bcd0",
            vec![],
            "wip yak",
            "wip",
            "2025-02-19",
            "Matt Wynne",
            vec![],
            vec![],
            vec![],
        );
        let terminal = draw_test_show_header_box(60, &view);

        let buf = terminal.backend().buffer();
        let mut found_bold = false;
        for x in 0..buf.area.width {
            let cell = &buf[(x, 1)];
            if cell.symbol() == "w" && buf[(x + 1, 1)].symbol() == "i" {
                found_bold = cell.modifier.contains(Modifier::BOLD);
                break;
            }
        }
        assert!(found_bold, "Expected BOLD modifier on wip yak name");
    }

    #[test]
    fn header_box_dim_style_on_done_name() {
        let view = make_detail_view(
            "done-yak-efg1",
            vec![],
            "done yak",
            "done",
            "2025-02-19",
            "Matt Wynne",
            vec![],
            vec![],
            vec![],
        );
        let terminal = draw_test_show_header_box(60, &view);

        let buf = terminal.backend().buffer();
        let mut found_strikethrough = false;
        for x in 0..buf.area.width {
            let cell = &buf[(x, 1)];
            if cell.symbol() == "d"
                && buf[(x + 1, 1)].symbol() == "o"
                && cell.modifier.contains(Modifier::CROSSED_OUT)
            {
                found_strikethrough = true;
                break;
            }
        }
        assert!(
            found_strikethrough,
            "Expected CROSSED_OUT modifier on done yak name"
        );
    }
}
