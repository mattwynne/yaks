// TUI adapter - implementation using ratatui for rich terminal output

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::narrative::NarrativeSpan;
use crate::domain::slug::Name;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Terminal, TerminalOptions, Viewport};
use std::io;

pub struct TuiDisplay {
    width: usize,
}

impl TuiDisplay {
    pub fn new(width: usize) -> Self {
        Self { width }
    }

    pub fn stdout() -> Self {
        let width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);
        Self { width }
    }

    /// Render the header box into a ratatui Buffer for the given area.
    /// This is the core rendering logic, extracted so tests can call it
    /// directly with a test buffer.
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
}

impl crate::domain::ports::DisplayPort for TuiDisplay {
    fn width(&self) -> usize {
        self.width
    }

    fn display_header_box(
        &self,
        ancestors: &[Name],
        name: &Name,
        state: &str,
        created_at: &Timestamp,
        created_by: &Author,
        children: &[(Name, String)],
        fields: &[(String, String)],
        tags: &[String],
    ) {
        // Calculate needed height:
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

        let backend = CrosstermBackend::new(io::stdout());
        let Ok(mut terminal) = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(height),
            },
        ) else {
            return;
        };

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
        println!();
    }

    // --- Stub implementations for remaining DisplayPort methods ---
    // These delegate to simple stdout writes. They don't need ratatui yet.

    fn display_hint(&self, message: &str) {
        for line in message.lines() {
            println!("  {line}");
        }
    }

    fn success(&self, message: &str) {
        println!("{message}");
    }

    fn info(&self, message: &str) {
        println!("{message}");
    }

    fn warn(&self, message: &str) {
        eprintln!("Warning: {message}");
    }

    fn display_yak_pretty(&self, prefix: &str, name: &Name, state: &str, tags: &[String]) {
        let tag_suffix = if tags.is_empty() {
            String::new()
        } else {
            format!(" {}", tags.join(" "))
        };
        let indicator = match state {
            "wip" | "done" => "●",
            _ => "○",
        };
        println!("{prefix}{indicator} {name}{tag_suffix}");
    }

    fn display_yak_markdown(&self, depth: usize, name: &Name, state: &str, tags: &[String]) {
        let indent = "  ".repeat(depth);
        let tag_suffix = if tags.is_empty() {
            String::new()
        } else {
            format!(" {}", tags.join(" "))
        };
        println!("{indent}- [{state}] {name}{tag_suffix}");
    }

    fn display_breadcrumb(&self, ancestors: &[Name]) {
        if ancestors.is_empty() {
            return;
        }
        let path = ancestors
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" > ");
        println!("{path} > ");
    }

    fn display_section_rule(&self, label: &str) {
        let header = format!("── {label} ");
        let padding = self.width.saturating_sub(header.chars().count());
        println!("{header}{}", "─".repeat(padding));
    }

    fn display_closing_rule(&self) {
        println!("{}", "─".repeat(self.width));
    }

    fn display_context(&self, context: &str) {
        for line in context.lines() {
            println!("  {line}");
        }
    }

    fn display_metadata_line(&self, state: &str, created_at: &Timestamp, created_by: &Author) {
        let date = chrono::DateTime::from_timestamp(created_at.as_epoch_secs(), 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("State: {state} · Created: {date} by {}", created_by.name);
    }

    fn log_entry(
        &self,
        narrative: &[NarrativeSpan],
        timestamp: &str,
        event_id: &str,
        commit_sha: Option<&str>,
    ) {
        let rendered = crate::domain::narrative::to_plain_text(narrative);
        let sha_part = match commit_sha {
            Some(sha) if sha.len() >= 7 => format!("  sha: {}", &sha[..7]),
            Some(sha) => format!("  sha: {sha}"),
            None => String::new(),
        };
        let rule: String = "─".repeat(self.width);
        println!("{rendered}");
        println!("{timestamp}");
        println!("event: {event_id}{sha_part}");
        println!("{rule}");
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
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn make_tui_display(width: usize) -> TuiDisplay {
        TuiDisplay::new(width)
    }

    #[test]
    fn header_box_renders_box_drawing_borders() {
        let display = make_tui_display(60);
        let name = Name::from("my yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt Wynne".to_string(),
            email: "matt@example.com".to_string(),
        };

        let area = Rect::new(0, 0, 60, 3);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &[],
            &name,
            "wip",
            &timestamp,
            &author,
            &[],
            &[],
            &[],
            area,
            &mut buf,
        );

        let output = buffer_to_string(&buf);
        let lines: Vec<&str> = output.lines().collect();

        // Top border
        assert!(
            lines[0].starts_with('┌'),
            "Expected top-left corner, got: {:?}",
            lines[0]
        );
        assert!(
            lines[0].ends_with('┐'),
            "Expected top-right corner, got: {:?}",
            lines[0]
        );

        // Header content
        assert!(
            lines[1].contains("● my yak"),
            "Expected indicator + name, got: {:?}",
            lines[1]
        );
        assert!(
            lines[1].contains("wip"),
            "Expected state, got: {:?}",
            lines[1]
        );
        assert!(
            lines[1].contains("2025-02-19"),
            "Expected date, got: {:?}",
            lines[1]
        );
        assert!(
            lines[1].contains("Matt Wynne"),
            "Expected author, got: {:?}",
            lines[1]
        );

        // Bottom border
        assert!(
            lines[2].starts_with('└'),
            "Expected bottom-left corner, got: {:?}",
            lines[2]
        );
        assert!(
            lines[2].ends_with('┘'),
            "Expected bottom-right corner, got: {:?}",
            lines[2]
        );
    }

    #[test]
    fn header_box_shows_breadcrumb_for_ancestors() {
        let display = make_tui_display(60);
        let name = Name::from("child yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt".to_string(),
            email: "m@e.com".to_string(),
        };
        let ancestors = vec![Name::from("parent")];

        // Need extra row for breadcrumb
        let area = Rect::new(0, 0, 60, 4);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &ancestors,
            &name,
            "todo",
            &timestamp,
            &author,
            &[],
            &[],
            &[],
            area,
            &mut buf,
        );

        let output = buffer_to_string(&buf);
        assert!(
            output.contains("parent >"),
            "Expected breadcrumb, got:\n{output}"
        );
        assert!(
            output.contains("child yak"),
            "Expected name, got:\n{output}"
        );
    }

    #[test]
    fn header_box_shows_children() {
        let display = make_tui_display(60);
        let name = Name::from("parent yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt".to_string(),
            email: "m@e.com".to_string(),
        };
        let children = vec![
            (Name::from("child one"), "todo".to_string()),
            (Name::from("child two"), "wip".to_string()),
        ];

        let area = Rect::new(0, 0, 60, 5);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &[],
            &name,
            "todo",
            &timestamp,
            &author,
            &children,
            &[],
            &[],
            area,
            &mut buf,
        );

        let output = buffer_to_string(&buf);
        assert!(
            output.contains("├─"),
            "Expected tree connector, got:\n{output}"
        );
        assert!(
            output.contains("╰─"),
            "Expected last-child connector, got:\n{output}"
        );
        assert!(
            output.contains("child one"),
            "Expected first child, got:\n{output}"
        );
        assert!(
            output.contains("child two"),
            "Expected second child, got:\n{output}"
        );
    }

    #[test]
    fn header_box_shows_fields() {
        let display = make_tui_display(60);
        let name = Name::from("my yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt".to_string(),
            email: "m@e.com".to_string(),
        };
        let fields = vec![
            ("worktree".to_string(), "/tmp/wt".to_string()),
            ("branch".to_string(), "feat-x".to_string()),
        ];

        // borders(2) + header(1) + divider(1) + fields(2) = 6
        let area = Rect::new(0, 0, 60, 6);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &[],
            &name,
            "wip",
            &timestamp,
            &author,
            &[],
            &fields,
            &[],
            area,
            &mut buf,
        );

        let output = buffer_to_string(&buf);
        assert!(
            output.contains("worktree: /tmp/wt"),
            "Expected worktree field, got:\n{output}"
        );
        assert!(
            output.contains("branch: feat-x"),
            "Expected branch field, got:\n{output}"
        );
    }

    #[test]
    fn header_box_shows_tags() {
        let display = make_tui_display(80);
        let name = Name::from("tagged yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt".to_string(),
            email: "m@e.com".to_string(),
        };
        let tags = vec!["@bug".to_string(), "@urgent".to_string()];

        let area = Rect::new(0, 0, 80, 3);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &[],
            &name,
            "todo",
            &timestamp,
            &author,
            &[],
            &[],
            &tags,
            area,
            &mut buf,
        );

        let output = buffer_to_string(&buf);
        assert!(output.contains("@bug"), "Expected @bug tag, got:\n{output}");
        assert!(
            output.contains("@urgent"),
            "Expected @urgent tag, got:\n{output}"
        );
    }

    #[test]
    fn header_box_done_state_uses_filled_indicator() {
        let display = make_tui_display(60);
        let name = Name::from("done yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt".to_string(),
            email: "m@e.com".to_string(),
        };

        let area = Rect::new(0, 0, 60, 3);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &[],
            &name,
            "done",
            &timestamp,
            &author,
            &[],
            &[],
            &[],
            area,
            &mut buf,
        );

        let output = buffer_to_string(&buf);
        assert!(
            output.contains("● done yak"),
            "Expected filled indicator for done state, got:\n{output}"
        );
    }

    #[test]
    fn header_box_todo_state_uses_open_indicator() {
        let display = make_tui_display(60);
        let name = Name::from("todo yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt".to_string(),
            email: "m@e.com".to_string(),
        };

        let area = Rect::new(0, 0, 60, 3);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &[],
            &name,
            "todo",
            &timestamp,
            &author,
            &[],
            &[],
            &[],
            area,
            &mut buf,
        );

        let output = buffer_to_string(&buf);
        assert!(
            output.contains("○ todo yak"),
            "Expected open indicator for todo state, got:\n{output}"
        );
    }

    #[test]
    fn header_box_bold_style_on_wip_name() {
        let display = make_tui_display(60);
        let name = Name::from("wip yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt".to_string(),
            email: "m@e.com".to_string(),
        };

        let area = Rect::new(0, 0, 60, 3);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &[],
            &name,
            "wip",
            &timestamp,
            &author,
            &[],
            &[],
            &[],
            area,
            &mut buf,
        );

        // Check that the 'w' in "wip yak" has BOLD modifier
        // The name starts after "│ ● " which is 4 cells from the left
        // Find the name cell
        let mut found_bold = false;
        for x in 0..area.width {
            let cell = &buf[(x, 1)];
            if cell.symbol() == "w" {
                // Check the next cells form "wip yak"
                let next = &buf[(x + 1, 1)];
                if next.symbol() == "i" {
                    found_bold = cell.modifier.contains(Modifier::BOLD);
                    break;
                }
            }
        }
        assert!(found_bold, "Expected BOLD modifier on wip yak name");
    }

    #[test]
    fn header_box_dim_style_on_done_name() {
        let display = make_tui_display(60);
        let name = Name::from("done yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt".to_string(),
            email: "m@e.com".to_string(),
        };

        let area = Rect::new(0, 0, 60, 3);
        let mut buf = Buffer::empty(area);

        display.render_header_box(
            &[],
            &name,
            "done",
            &timestamp,
            &author,
            &[],
            &[],
            &[],
            area,
            &mut buf,
        );

        // Check that the 'd' in "done yak" (the name) has CROSSED_OUT
        let mut found_strikethrough = false;
        for x in 0..area.width {
            let cell = &buf[(x, 1)];
            if cell.symbol() == "d" {
                let next = &buf[(x + 1, 1)];
                if next.symbol() == "o" {
                    // Check for the name "done yak", not the state "done"
                    let next2 = &buf[(x + 2, 1)];
                    let next3 = &buf[(x + 3, 1)];
                    if next2.symbol() == "n" && next3.symbol() == "e" {
                        // Could be the name or the state; check if CROSSED_OUT
                        if cell.modifier.contains(Modifier::CROSSED_OUT) {
                            found_strikethrough = true;
                            break;
                        }
                    }
                }
            }
        }
        assert!(
            found_strikethrough,
            "Expected CROSSED_OUT modifier on done yak name"
        );
    }
}
