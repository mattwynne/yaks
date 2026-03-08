// TUI adapter - implementation using ratatui for rich terminal output

use crate::adapters::user_display::ConsoleDisplay;
use crate::adapters::views::{LogEntryView, Message, YakDetailView, YakTreeView};
use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::slug::Name;
use ratatui::backend::Backend;
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

    #[allow(dead_code)]
    fn header_box_height(
        ancestors: &[Name],
        children: &[(Name, String)],
        fields: &[(String, String)],
    ) -> u16 {
        // 2 for top/bottom border
        // 1 for header line
        // 1 for breadcrumb if ancestors present
        // children count
        // 1 for divider + field count if fields present
        let mut height: u16 = 2 + 1; // borders + header
        if !ancestors.is_empty() {
            height += 1;
        }
        height += children.len() as u16;
        if !fields.is_empty() {
            height += 1 + fields.len() as u16; // divider + fields
        }
        height
    }

    /// Draw the header box through a Terminal, handling draw + cursor
    /// cleanup. Generic over backend so tests can use TestBackend.
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    fn draw_header_box<B: Backend>(
        &self,
        terminal: &mut Terminal<B>,
        ancestors: &[Name],
        name: &Name,
        state: &str,
        created_at: &Timestamp,
        created_by: &Author,
        children: &[(Name, String)],
        fields: &[(String, String)],
        tags: &[String],
    ) {
        let _ = terminal.draw(|frame| {
            let area = frame.area();
            self.render_header_box(
                ancestors,
                name,
                state,
                created_at,
                created_by,
                children,
                fields,
                tags,
                area,
                frame.buffer_mut(),
            );
        });
        let _ = terminal.show_cursor();
    }

    /// Render the header box into a ratatui Buffer for the given area.
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    fn render_header_box(
        &self,
        ancestors: &[Name],
        name: &Name,
        state: &str,
        created_at: &Timestamp,
        created_by: &Author,
        children: &[(Name, String)],
        fields: &[(String, String)],
        tags: &[String],
        area: Rect,
        buf: &mut ratatui::buffer::Buffer,
    ) {
        fn indicator_for(state: &str) -> &'static str {
            match state {
                "wip" | "done" => "●",
                _ => "○",
            }
        }

        fn state_color(state: &str) -> Color {
            match state {
                "wip" => Color::Green,
                "done" => Color::DarkGray,
                _ => Color::Reset,
            }
        }

        let indicator = indicator_for(state);
        let date = chrono::DateTime::from_timestamp(created_at.as_epoch_secs(), 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Build the lines for the Paragraph
        let mut lines: Vec<Line> = Vec::new();

        // Breadcrumb line (if ancestors exist)
        if !ancestors.is_empty() {
            let path = ancestors
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(" > ");
            lines.push(Line::from(Span::styled(
                format!(" {path} >"),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }

        // Header line: indicator + name + state + date + author + tags
        let mut header_spans: Vec<Span> = Vec::new();
        header_spans.push(Span::raw(" "));

        // Indicator with state color
        header_spans.push(Span::styled(
            indicator,
            Style::default().fg(state_color(state)),
        ));
        header_spans.push(Span::raw(" "));

        // Name (bold, or strikethrough+dim for done)
        match state {
            "done" => {
                header_spans.push(Span::styled(
                    name.to_string(),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::CROSSED_OUT),
                ));
            }
            _ => {
                header_spans.push(Span::styled(
                    name.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
        }

        // Metadata: state, date, author (dimmed)
        header_spans.push(Span::styled(
            format!(" · {state} · {date} · {}", created_by.name),
            Style::default().fg(Color::DarkGray),
        ));

        // Tags
        if !tags.is_empty() {
            header_spans.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            header_spans.push(Span::styled(
                tags.join(" "),
                Style::default().fg(Color::Rgb(95, 135, 175)), // ~38;5;67
            ));
        }

        lines.push(Line::from(header_spans));

        // Children
        for (i, (cname, cstate)) in children.iter().enumerate() {
            let connector = if i == children.len() - 1 {
                "╰─"
            } else {
                "├─"
            };
            let ci = indicator_for(cstate);
            let mut child_spans: Vec<Span> = Vec::new();
            child_spans.push(Span::raw(format!(" {connector} ")));
            child_spans.push(Span::styled(ci, Style::default().fg(state_color(cstate))));
            child_spans.push(Span::raw(" "));
            match cstate.as_str() {
                "done" => {
                    child_spans.push(Span::styled(
                        cname.to_string(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::CROSSED_OUT),
                    ));
                }
                "wip" => {
                    child_spans.push(Span::styled(
                        cname.to_string(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                }
                _ => {
                    child_spans.push(Span::raw(cname.to_string()));
                }
            }
            lines.push(Line::from(child_spans));
        }

        // Fields section: show as rows after a divider
        if !fields.is_empty() {
            // Divider line
            let divider_width = area.width.saturating_sub(2) as usize;
            lines.push(Line::from(Span::styled(
                "─".repeat(divider_width),
                Style::default().add_modifier(Modifier::DIM),
            )));

            let max_label_width = fields
                .iter()
                .map(|(k, _)| k.chars().count())
                .max()
                .unwrap_or(0);
            for (k, v) in fields {
                let pad = max_label_width - k.chars().count();
                lines.push(Line::from(format!(" {}{}: {}", " ".repeat(pad), k, v)));
            }
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().add_modifier(Modifier::DIM));

        let paragraph = Paragraph::new(lines).block(block);
        ratatui::widgets::Widget::render(paragraph, area, buf);
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
                "wip" | "done" => "●",
                _ => "○",
            }
        }

        fn state_color(state: &str) -> Color {
            match state {
                "wip" => Color::Green,
                "done" => Color::DarkGray,
                _ => Color::Reset,
            }
        }

        let state = view.state.as_str();
        let indicator = indicator_for(state);
        let mut lines: Vec<Line> = Vec::new();

        // Breadcrumb
        if !view.breadcrumb.is_empty() {
            let path = view.breadcrumb.join(" > ");
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
    use crate::domain::event_metadata::{Author, Timestamp};
    use crate::domain::slug::Name;
    use ratatui::backend::TestBackend;

    /// Create a TestBackend terminal sized for the given header box
    /// inputs, then call draw_header_box through it. Returns the
    /// terminal so tests can inspect the buffer.
    #[allow(clippy::too_many_arguments)]
    fn draw_test_header_box(
        width: u16,
        ancestors: &[Name],
        name: &Name,
        state: &str,
        created_at: &Timestamp,
        created_by: &Author,
        children: &[(Name, String)],
        fields: &[(String, String)],
        tags: &[String],
    ) -> Terminal<TestBackend> {
        let height = TuiDisplay::header_box_height(ancestors, children, fields);
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, width, height)),
            },
        )
        .unwrap();
        let display = TuiDisplay::new(width as usize);
        display.draw_header_box(
            &mut terminal,
            ancestors,
            name,
            state,
            created_at,
            created_by,
            children,
            fields,
            tags,
        );
        terminal
    }

    fn ts() -> Timestamp {
        Timestamp(1739923200)
    }

    fn author() -> Author {
        Author {
            name: "Matt Wynne".to_string(),
            email: "matt@example.com".to_string(),
        }
    }

    #[test]
    fn header_box_renders_box_drawing_borders() {
        let terminal = draw_test_header_box(
            60,
            &[],
            &Name::from("my yak"),
            "wip",
            &ts(),
            &author(),
            &[],
            &[],
            &[],
        );

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
        let ancestors = vec![Name::from("parent")];
        let terminal = draw_test_header_box(
            60,
            &ancestors,
            &Name::from("child yak"),
            "todo",
            &ts(),
            &author(),
            &[],
            &[],
            &[],
        );

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("parent >"), "got:\n{output}");
        assert!(output.contains("child yak"), "got:\n{output}");
    }

    #[test]
    fn header_box_shows_children() {
        let children = vec![
            (Name::from("child one"), "todo".to_string()),
            (Name::from("child two"), "wip".to_string()),
        ];
        let terminal = draw_test_header_box(
            60,
            &[],
            &Name::from("parent yak"),
            "todo",
            &ts(),
            &author(),
            &children,
            &[],
            &[],
        );

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("├─"), "got:\n{output}");
        assert!(output.contains("╰─"), "got:\n{output}");
        assert!(output.contains("child one"), "got:\n{output}");
        assert!(output.contains("child two"), "got:\n{output}");
    }

    #[test]
    fn header_box_shows_fields() {
        let fields = vec![
            ("worktree".to_string(), "/tmp/wt".to_string()),
            ("branch".to_string(), "feat-x".to_string()),
        ];
        let terminal = draw_test_header_box(
            60,
            &[],
            &Name::from("my yak"),
            "wip",
            &ts(),
            &author(),
            &[],
            &fields,
            &[],
        );

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("worktree: /tmp/wt"), "got:\n{output}");
        assert!(output.contains("branch: feat-x"), "got:\n{output}");
    }

    #[test]
    fn header_box_shows_tags() {
        let tags = vec!["@bug".to_string(), "@urgent".to_string()];
        let terminal = draw_test_header_box(
            80,
            &[],
            &Name::from("tagged yak"),
            "todo",
            &ts(),
            &author(),
            &[],
            &[],
            &tags,
        );

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("@bug"), "got:\n{output}");
        assert!(output.contains("@urgent"), "got:\n{output}");
    }

    #[test]
    fn header_box_done_state_uses_filled_indicator() {
        let terminal = draw_test_header_box(
            60,
            &[],
            &Name::from("done yak"),
            "done",
            &ts(),
            &author(),
            &[],
            &[],
            &[],
        );

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("● done yak"), "got:\n{output}");
    }

    #[test]
    fn header_box_todo_state_uses_open_indicator() {
        let terminal = draw_test_header_box(
            60,
            &[],
            &Name::from("todo yak"),
            "todo",
            &ts(),
            &author(),
            &[],
            &[],
            &[],
        );

        let output = buffer_to_string(terminal.backend().buffer());
        assert!(output.contains("○ todo yak"), "got:\n{output}");
    }

    #[test]
    fn header_box_bold_style_on_wip_name() {
        let terminal = draw_test_header_box(
            60,
            &[],
            &Name::from("wip yak"),
            "wip",
            &ts(),
            &author(),
            &[],
            &[],
            &[],
        );

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
        let terminal = draw_test_header_box(
            60,
            &[],
            &Name::from("done yak"),
            "done",
            &ts(),
            &author(),
            &[],
            &[],
            &[],
        );

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

    #[test]
    fn header_box_height_no_ancestors_no_children_no_fields() {
        assert_eq!(TuiDisplay::header_box_height(&[], &[], &[]), 3);
    }

    #[test]
    fn header_box_height_with_ancestors() {
        let ancestors = vec![Name::from("parent")];
        assert_eq!(TuiDisplay::header_box_height(&ancestors, &[], &[]), 4);
    }

    #[test]
    fn header_box_height_with_children_and_fields() {
        let children = vec![
            (Name::from("a"), "todo".to_string()),
            (Name::from("b"), "wip".to_string()),
        ];
        let fields = vec![("key".to_string(), "val".to_string())];
        assert_eq!(
            TuiDisplay::header_box_height(&[], &children, &fields),
            3 + 2 + 1 + 1 // base + children + divider + field
        );
    }
}
