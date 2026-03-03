pub mod relative_time;
// CLI adapter - implementation using clap

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::narrative::NarrativeSpan;
use crate::domain::slug::Name;
use std::io::{IsTerminal, Write};
use std::sync::Mutex;

pub struct ConsoleDisplayOptions {
    pub color: bool,
    pub width: usize,
}

pub struct ConsoleDisplay {
    output: Mutex<Box<dyn Write + Send>>,
    options: ConsoleDisplayOptions,
}

impl ConsoleDisplay {
    pub fn new(output: Box<dyn Write + Send>, options: ConsoleDisplayOptions) -> Self {
        Self {
            output: Mutex::new(output),
            options,
        }
    }

    pub fn stdout() -> Self {
        let width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);
        let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
        Self::new(
            Box::new(std::io::stdout()),
            ConsoleDisplayOptions { color, width },
        )
    }

    /// Render narrative spans to a string, with optional ANSI bold for highlights.
    fn render_narrative(&self, narrative: &[NarrativeSpan]) -> String {
        if self.options.color {
            narrative
                .iter()
                .map(|span| match span {
                    NarrativeSpan::Plain(t) => t.clone(),
                    NarrativeSpan::Highlight(t) => format!("\x1b[1m{t}\x1b[0m"),
                })
                .collect()
        } else {
            crate::domain::narrative::to_plain_text(narrative)
        }
    }
}

/// Helper for rendering styled yak items (name + indicator) consistently
fn style_yak_item(name: &str, state: &str, color: bool) -> String {
    let indicator = match state {
        "wip" | "done" => "●",
        _ => "○",
    };
    if color {
        match state {
            "wip" => format!("\x1b[32m●\x1b[0m \x1b[1m{name}\x1b[0m"),
            "done" => format!("\x1b[90m●\x1b[0m \x1b[90;9m{name}\x1b[0m"),
            _ => format!("○ \x1b[1m{name}\x1b[0m"),
        }
    } else {
        format!("{indicator} {name}")
    }
}

/// Helper to write a padded line inside the box
fn write_box_line(
    out: &mut Box<dyn Write + Send>,
    content: &str,
    visible_width: usize,
    inner_width: usize,
    color: bool,
) {
    let pad = inner_width - visible_width;
    if color {
        writeln!(
            out,
            "\x1b[2m│\x1b[0m{content}{}\x1b[2m│\x1b[0m",
            " ".repeat(pad)
        )
        .unwrap();
    } else {
        writeln!(out, "│{content}{}│", " ".repeat(pad)).unwrap();
    }
}

/// Helper to write a dimmed line inside the box
fn write_dim_line(
    out: &mut Box<dyn Write + Send>,
    content: &str,
    visible_width: usize,
    inner_width: usize,
    color: bool,
) {
    let pad = inner_width - visible_width;
    if color {
        writeln!(out, "\x1b[2m│{content}{}│\x1b[0m", " ".repeat(pad)).unwrap();
    } else {
        writeln!(out, "│{content}{}│", " ".repeat(pad)).unwrap();
    }
}

impl crate::domain::ports::DisplayPort for ConsoleDisplay {
    fn width(&self) -> usize {
        self.options.width
    }

    fn display_hint(&self, message: &str) {
        let mut out = self.output.lock().unwrap();
        if self.options.color {
            for line in message.lines() {
                writeln!(out, "  \x1b[3;90m{line}\x1b[0m").unwrap();
            }
        } else {
            for line in message.lines() {
                writeln!(out, "  {line}").unwrap();
            }
        }
    }

    fn success(&self, message: &str) {
        let mut out = self.output.lock().unwrap();
        writeln!(out, "{message}").unwrap();
    }

    fn info(&self, message: &str) {
        let mut out = self.output.lock().unwrap();
        writeln!(out, "{message}").unwrap();
    }

    fn warn(&self, message: &str) {
        eprintln!("Warning: {message}");
    }

    fn display_yak_pretty(&self, prefix: &str, name: &Name, state: &str, tags: &[String]) {
        let mut out = self.output.lock().unwrap();
        let tag_suffix = if tags.is_empty() {
            String::new()
        } else {
            format!(" {}", tags.join(" "))
        };
        if self.options.color {
            let dim_tags = if tag_suffix.is_empty() {
                String::new()
            } else {
                format!("\x1b[38;5;67m{}\x1b[0m", tag_suffix)
            };
            match state {
                "wip" => writeln!(
                    out,
                    "{prefix}\x1b[32m●\x1b[0m \x1b[1m{name}\x1b[0m{dim_tags}"
                ),
                "done" => writeln!(
                    out,
                    "{prefix}\x1b[90m●\x1b[0m \x1b[90;9m{name}\x1b[0m{dim_tags}"
                ),
                _ => writeln!(out, "{prefix}○ {name}{dim_tags}"),
            }
        } else {
            let indicator = match state {
                "wip" | "done" => "●",
                _ => "○",
            };
            writeln!(out, "{prefix}{indicator} {name}{tag_suffix}")
        }
        .unwrap();
    }

    fn display_yak_markdown(&self, depth: usize, name: &Name, state: &str, tags: &[String]) {
        let mut out = self.output.lock().unwrap();
        let indent = "  ".repeat(depth);
        let tag_suffix = if tags.is_empty() {
            String::new()
        } else {
            format!(" {}", tags.join(" "))
        };
        let line = format!("{indent}- [{state}] {name}{tag_suffix}");
        if self.options.color && state == "done" {
            writeln!(out, "\x1b[90m{line}\x1b[0m")
        } else {
            writeln!(out, "{line}")
        }
        .unwrap();
    }

    #[allow(clippy::cognitive_complexity)]
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
        let mut out = self.output.lock().unwrap();

        let indicator = match state {
            "wip" | "done" => "●",
            _ => "○",
        };
        let date = chrono::DateTime::from_timestamp(created_at.as_epoch_secs(), 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Build breadcrumb line (if any ancestors)
        let breadcrumb = if ancestors.is_empty() {
            None
        } else {
            let path = ancestors
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(" > ");
            Some(format!("  {path} >   "))
        };

        let tags_suffix = if tags.is_empty() {
            String::new()
        } else {
            format!(" · {}", tags.join(" "))
        };
        let header_content = format!(
            "  {indicator} {name} · {state} · {date} · {}{tags_suffix}  ",
            created_by.name
        );
        let header_width = header_content.chars().count();

        // Build child lines
        let child_lines: Vec<String> = children
            .iter()
            .enumerate()
            .map(|(i, (cname, cstate))| {
                let connector = if i == children.len() - 1 {
                    "╰─"
                } else {
                    "├─"
                };
                let ci = match cstate.as_str() {
                    "wip" | "done" => "●",
                    _ => "○",
                };
                format!("  {connector} {ci} {cname}  ")
            })
            .collect();

        // Build field lines with right-aligned labels
        let max_label_width = fields
            .iter()
            .map(|(k, _)| k.chars().count())
            .max()
            .unwrap_or(0);
        let field_lines: Vec<String> = fields
            .iter()
            .map(|(k, v)| {
                let pad = max_label_width - k.chars().count();
                format!("  {}{}: {}  ", " ".repeat(pad), k, v)
            })
            .collect();

        // Inner width = max of all lines and terminal width - 2
        let max_content_width = std::iter::once(header_width)
            .chain(breadcrumb.iter().map(|b| b.chars().count()))
            .chain(child_lines.iter().map(|l| l.chars().count()))
            .chain(field_lines.iter().map(|l| l.chars().count()))
            .max()
            .unwrap();
        let inner_width = (self.options.width.saturating_sub(2)).max(max_content_width);

        let top = format!("┌{}┐", "─".repeat(inner_width));
        let divider = format!("├{}┤", "─".repeat(inner_width));
        let bottom = format!("└{}┘", "─".repeat(inner_width));
        let color = self.options.color;

        // Top border
        if color {
            writeln!(out, "\x1b[2m{top}\x1b[0m").unwrap();
        } else {
            writeln!(out, "{top}").unwrap();
        }

        // Breadcrumb (dimmed)
        if let Some(ref bc) = breadcrumb {
            write_dim_line(&mut out, bc, bc.chars().count(), inner_width, color);
        }

        // Name line
        if color {
            let colored_tags = if tags.is_empty() {
                String::new()
            } else {
                format!("\x1b[90m ·\x1b[0m \x1b[38;5;67m{}\x1b[0m", tags.join(" "))
            };
            let meta = format!(
                "\x1b[90m · {state} · {date} · {}\x1b[0m{colored_tags}  ",
                created_by.name
            );
            let styled_header = format!("  {}{meta}", style_yak_item(name.as_ref(), state, true));
            write_box_line(&mut out, &styled_header, header_width, inner_width, true);
        } else {
            write_box_line(&mut out, &header_content, header_width, inner_width, false);
        }

        // Tags are now shown inline in the header line

        // Children
        for (i, (cname, cstate)) in children.iter().enumerate() {
            let connector = if i == children.len() - 1 {
                "╰─"
            } else {
                "├─"
            };
            if color {
                let styled_child = format!(
                    "  {connector} {}  ",
                    style_yak_item(cname.as_ref(), cstate, true)
                );
                write_box_line(
                    &mut out,
                    &styled_child,
                    child_lines[i].chars().count(),
                    inner_width,
                    true,
                );
            } else {
                write_box_line(
                    &mut out,
                    &child_lines[i],
                    child_lines[i].chars().count(),
                    inner_width,
                    false,
                );
            }
        }

        // Divider + fields
        if !fields.is_empty() {
            if color {
                writeln!(out, "\x1b[2m{divider}\x1b[0m").unwrap();
            } else {
                writeln!(out, "{divider}").unwrap();
            }
            for line in &field_lines {
                write_box_line(&mut out, line, line.chars().count(), inner_width, color);
            }
        }

        // Bottom border
        if color {
            writeln!(out, "\x1b[2m{bottom}\x1b[0m").unwrap();
        } else {
            writeln!(out, "{bottom}").unwrap();
        }
    }

    fn display_section_rule(&self, label: &str) {
        let mut out = self.output.lock().unwrap();
        let header = format!("── {label} ");
        let padding = self.options.width.saturating_sub(header.chars().count());
        let line = format!("{header}{}", "─".repeat(padding));
        if self.options.color {
            writeln!(out, "\x1b[2m{line}\x1b[0m").unwrap();
        } else {
            writeln!(out, "{line}").unwrap();
        }
    }

    fn display_closing_rule(&self) {
        let mut out = self.output.lock().unwrap();
        let line = "─".repeat(self.options.width);
        if self.options.color {
            writeln!(out, "\x1b[2m{line}\x1b[0m").unwrap();
        } else {
            writeln!(out, "{line}").unwrap();
        }
    }

    fn display_context(&self, context: &str) {
        let mut out = self.output.lock().unwrap();
        if self.options.color {
            let mut skin = termimad::MadSkin::default();
            skin.headers[0].align = termimad::Alignment::Left;
            let text = skin.term_text(context);
            // Indent each line by 2 spaces
            for line in format!("{text}").lines() {
                writeln!(out, "  {line}").unwrap();
            }
        } else {
            for line in context.lines() {
                writeln!(out, "  {line}").unwrap();
            }
        }
    }

    fn display_breadcrumb(&self, ancestors: &[Name]) {
        if ancestors.is_empty() {
            return;
        }
        let mut out = self.output.lock().unwrap();
        let path = ancestors
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(" > ");
        let line = format!("{path} > ");
        if self.options.color {
            writeln!(out, "\x1b[2m{line}\x1b[0m").unwrap();
        } else {
            writeln!(out, "{line}").unwrap();
        }
    }

    fn display_metadata_line(&self, state: &str, created_at: &Timestamp, created_by: &Author) {
        let mut out = self.output.lock().unwrap();
        let date = chrono::DateTime::from_timestamp(created_at.as_epoch_secs(), 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let line = format!("State: {state} · Created: {date} by {}", created_by.name);
        writeln!(out, "{line}").unwrap();
    }

    fn log_entry(
        &self,
        narrative: &[NarrativeSpan],
        timestamp: &str,
        event_id: &str,
        commit_sha: Option<&str>,
    ) {
        let mut out = self.output.lock().unwrap();
        let rule: String = "─".repeat(self.options.width);
        let sha_part = match commit_sha {
            Some(sha) if sha.len() >= 7 => format!("  sha: {}", &sha[..7]),
            Some(sha) => format!("  sha: {sha}"),
            None => String::new(),
        };
        let rendered = self.render_narrative(narrative);
        if self.options.color {
            writeln!(out, "{rendered}").unwrap();
            writeln!(out, "\x1b[2m{timestamp}\x1b[0m").unwrap();
            writeln!(out, "\x1b[2;90mevent: {event_id}{sha_part}\x1b[0m").unwrap();
            writeln!(out, "\x1b[2m{rule}\x1b[0m").unwrap();
        } else {
            writeln!(out, "{rendered}").unwrap();
            writeln!(out, "{timestamp}").unwrap();
            writeln!(out, "event: {event_id}{sha_part}").unwrap();
            writeln!(out, "{rule}").unwrap();
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
mod test_buffer {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    /// Thread-safe, cloneable buffer for capturing display output in tests.
    #[derive(Clone)]
    pub struct TestBuffer(Arc<Mutex<Vec<u8>>>);

    impl TestBuffer {
        pub fn new() -> Self {
            Self(Arc::new(Mutex::new(Vec::new())))
        }

        /// Returns the buffer contents as a UTF-8 string.
        pub fn contents(&self) -> String {
            let data = self.0.lock().unwrap();
            String::from_utf8(data.clone()).unwrap()
        }

        /// Clears the buffer.
        pub fn clear(&self) {
            self.0.lock().unwrap().clear();
        }
    }

    impl Default for TestBuffer {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Write for TestBuffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.0.lock().unwrap().flush()
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use test_buffer::TestBuffer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::narrative::{highlight, plain};
    use crate::domain::ports::DisplayPort;

    fn make_display(color: bool) -> (ConsoleDisplay, TestBuffer) {
        let buffer = TestBuffer::new();
        let writer = buffer.clone();
        let display =
            ConsoleDisplay::new(Box::new(writer), ConsoleDisplayOptions { color, width: 60 });
        (display, buffer)
    }

    #[test]
    fn test_buffer_captures_writes() {
        let mut buffer = TestBuffer::new();
        buffer.write_all(b"hello").unwrap();
        assert_eq!(buffer.contents(), "hello");
    }

    #[test]
    fn test_buffer_clear() {
        let mut buffer = TestBuffer::new();
        buffer.write_all(b"hello").unwrap();
        buffer.clear();
        assert_eq!(buffer.contents(), "");
    }

    #[test]
    fn test_buffer_clone_shares_data() {
        let mut buffer = TestBuffer::new();
        let clone = buffer.clone();
        buffer.write_all(b"shared").unwrap();
        assert_eq!(clone.contents(), "shared");
    }

    #[test]
    fn success_writes_message() {
        let (display, buffer) = make_display(false);
        display.success("hello world");
        assert_eq!(buffer.contents(), "hello world\n");
    }

    #[test]
    fn info_writes_message() {
        let (display, buffer) = make_display(false);
        display.info("some info");
        assert_eq!(buffer.contents(), "some info\n");
    }

    #[test]
    fn pretty_wip_with_color_has_ansi() {
        let (display, buffer) = make_display(true);
        let name = Name::from("my yak");
        display.display_yak_pretty("", &name, "wip", &[]);
        let output = buffer.contents();
        assert!(output.contains("\x1b["), "expected ANSI codes in: {output}");
        assert!(output.contains("my yak"));
    }

    #[test]
    fn pretty_done_with_color_has_ansi() {
        let (display, buffer) = make_display(true);
        let name = Name::from("finished yak");
        display.display_yak_pretty("", &name, "done", &[]);
        let output = buffer.contents();
        assert!(output.contains("\x1b["), "expected ANSI codes in: {output}");
    }

    #[test]
    fn pretty_wip_without_color_has_no_ansi() {
        let (display, buffer) = make_display(false);
        let name = Name::from("my yak");
        display.display_yak_pretty("", &name, "wip", &[]);
        let output = buffer.contents();
        assert!(
            !output.contains("\x1b["),
            "unexpected ANSI codes in: {output}"
        );
        assert!(output.contains("●"));
        assert!(output.contains("my yak"));
    }

    #[test]
    fn pretty_done_without_color_has_no_ansi() {
        let (display, buffer) = make_display(false);
        let name = Name::from("done yak");
        display.display_yak_pretty("", &name, "done", &[]);
        let output = buffer.contents();
        assert!(
            !output.contains("\x1b["),
            "unexpected ANSI codes in: {output}"
        );
        assert!(output.contains("●"));
    }

    #[test]
    fn pretty_todo_without_color_uses_open_circle() {
        let (display, buffer) = make_display(false);
        let name = Name::from("todo yak");
        display.display_yak_pretty("", &name, "todo", &[]);
        let output = buffer.contents();
        assert!(
            !output.contains("\x1b["),
            "unexpected ANSI codes in: {output}"
        );
        assert!(output.contains("○"));
    }

    #[test]
    fn markdown_done_with_color_has_ansi() {
        let (display, buffer) = make_display(true);
        let name = Name::from("done yak");
        display.display_yak_markdown(0, &name, "done", &[]);
        let output = buffer.contents();
        assert!(
            output.contains("\x1b[90m"),
            "expected ANSI codes in: {output}"
        );
        assert!(output.contains("[done] done yak"));
    }

    #[test]
    fn markdown_done_without_color_has_no_ansi() {
        let (display, buffer) = make_display(false);
        let name = Name::from("done yak");
        display.display_yak_markdown(0, &name, "done", &[]);
        let output = buffer.contents();
        assert!(
            !output.contains("\x1b["),
            "unexpected ANSI codes in: {output}"
        );
        assert!(output.contains("- [done] done yak"));
    }

    #[test]
    fn markdown_todo_without_color_has_no_ansi() {
        let (display, buffer) = make_display(false);
        let name = Name::from("todo yak");
        display.display_yak_markdown(1, &name, "todo", &[]);
        let output = buffer.contents();
        assert!(
            !output.contains("\x1b["),
            "unexpected ANSI codes in: {output}"
        );
        assert!(output.contains("  - [todo] todo yak"));
    }

    #[test]
    fn header_box_renders_with_box_drawing_chars() {
        let (display, buffer) = make_display(false);
        let name = Name::from("my yak");
        let timestamp = Timestamp(1739923200);
        let author = Author {
            name: "Matt Wynne".to_string(),
            email: "matt@example.com".to_string(),
        };
        display.display_header_box(&[], &name, "wip", &timestamp, &author, &[], &[], &[]);
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        assert!(
            lines[0].starts_with('┌'),
            "Expected top border, got: {:?}",
            lines[0]
        );
        assert!(
            lines[0].ends_with('┐'),
            "Expected top border end, got: {:?}",
            lines[0]
        );
        assert!(
            lines[1].contains("● my yak"),
            "Expected name in box, got: {:?}",
            lines[1]
        );
        assert!(
            lines[1].contains("wip"),
            "Expected state in box, got: {:?}",
            lines[1]
        );
        assert!(
            lines[1].contains("2025-02-19"),
            "Expected date in box, got: {:?}",
            lines[1]
        );
        assert!(
            lines[1].contains("Matt Wynne"),
            "Expected author in box, got: {:?}",
            lines[1]
        );
        assert!(
            lines[2].starts_with('└'),
            "Expected bottom border, got: {:?}",
            lines[2]
        );
        assert!(
            lines[2].ends_with('┘'),
            "Expected bottom border end, got: {:?}",
            lines[2]
        );
    }

    #[test]
    fn breadcrumb_empty_ancestors_produces_no_output() {
        let (display, buffer) = make_display(false);
        display.display_breadcrumb(&[]);
        assert_eq!(buffer.contents(), "");
    }

    #[test]
    fn breadcrumb_single_ancestor() {
        let (display, buffer) = make_display(false);
        display.display_breadcrumb(&[Name::from("parent")]);
        assert_eq!(buffer.contents(), "parent > \n");
    }

    #[test]
    fn breadcrumb_multiple_ancestors() {
        let (display, buffer) = make_display(false);
        display.display_breadcrumb(&[Name::from("grandparent"), Name::from("parent")]);
        assert_eq!(buffer.contents(), "grandparent > parent > \n");
    }

    #[test]
    fn breadcrumb_with_color_is_dimmed() {
        let (display, buffer) = make_display(true);
        display.display_breadcrumb(&[Name::from("root"), Name::from("mid")]);
        let output = buffer.contents();
        assert!(
            output.contains("\x1b[2m"),
            "expected dim ANSI code in: {output}"
        );
        assert!(output.contains("root > mid > "));
        assert!(
            output.contains("\x1b[0m"),
            "expected reset ANSI code in: {output}"
        );
    }

    #[test]
    fn metadata_line_shows_state_date_and_author() {
        let (display, buffer) = make_display(false);
        let timestamp = Timestamp(1739923200); // 2025-02-19 00:00:00 UTC
        let author = Author {
            name: "Matt Wynne".to_string(),
            email: "matt@example.com".to_string(),
        };
        display.display_metadata_line("todo", &timestamp, &author);
        let output = buffer.contents();
        assert_eq!(output, "State: todo · Created: 2025-02-19 by Matt Wynne\n");
    }

    #[test]
    fn log_entry_no_color_renders_plain_text() {
        let (display, buffer) = make_display(false);
        let narrative = vec![highlight("Matt"), plain(" added "), highlight("my yak")];
        display.log_entry(&narrative, "just now", "abc123", None);
        let output = buffer.contents();
        assert!(
            output.contains("Matt added my yak"),
            "Expected plain narrative in: {output}"
        );
        assert!(
            !output.contains("\x1b["),
            "unexpected ANSI codes in: {output}"
        );
    }

    #[test]
    fn log_entry_with_color_renders_bold_highlights() {
        let (display, buffer) = make_display(true);
        let narrative = vec![highlight("Matt"), plain(" added "), highlight("my yak")];
        display.log_entry(&narrative, "just now", "abc123", None);
        let output = buffer.contents();
        assert!(
            output.contains("\x1b[1mMatt\x1b[0m"),
            "Expected bold author in: {output}"
        );
        assert!(
            output.contains("\x1b[1mmy yak\x1b[0m"),
            "Expected bold yak name in: {output}"
        );
        assert!(
            output.contains(" added "),
            "Expected plain 'added' in: {output}"
        );
    }
}

/// Create a `ConsoleDisplay` + `TestBuffer` pair for use in tests.
/// Output is plain text (no ANSI color codes).
#[cfg(any(test, feature = "test-support"))]
pub fn make_test_display() -> (ConsoleDisplay, TestBuffer) {
    let buffer = TestBuffer::new();
    let writer = buffer.clone();
    let display = ConsoleDisplay::new(
        Box::new(writer),
        ConsoleDisplayOptions {
            color: false,
            width: 60,
        },
    );
    (display, buffer)
}
