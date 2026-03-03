pub mod relative_time;
// CLI adapter - implementation using clap

use crate::domain::slug::Name;
use crate::domain::views::{LogEntryView, Message, YakDetailView, YakTreeNode, YakTreeView};
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

    /// Helper to recursively render tree nodes
    fn render_tree_nodes(&self, nodes: &[YakTreeNode], format: &str) {
        for node in nodes {
            match format {
                "plain" => self.info(&node.full_path),
                "ids" => self.info(&node.id),
                "pretty" => {
                    let node_prefix = format!(" {}{}", node.prefix, node.connector);
                    let name_ref = Name::from(node.name.as_str());
                    self.display_yak_pretty(&node_prefix, &name_ref, &node.state, &node.tags);
                }
                _ => {
                    // markdown
                    let name_ref = Name::from(node.name.as_str());
                    self.display_yak_markdown(node.depth, &name_ref, &node.state, &node.tags);
                }
            }
            self.render_tree_nodes(&node.children, format);
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

// Private helper methods (used by high-level DisplayPort methods)
impl ConsoleDisplay {
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
}

impl crate::domain::ports::DisplayPort for ConsoleDisplay {
    fn width(&self) -> usize {
        self.options.width
    }

    fn message(&self, msg: &Message) {
        match msg {
            Message::Hint(s) => self.display_hint(s),
            Message::Success(s) => self.success(s),
            Message::Info(s) => self.info(s),
            Message::Warn(s) => self.warn(s),
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn show_yak(&self, view: &YakDetailView) {
        // Render header box
        {
            let mut out = self.output.lock().unwrap();
            let color = self.options.color;
            let state = view.state.as_str();
            let name = &view.name;

            let indicator = match state {
                "wip" | "done" => "●",
                _ => "○",
            };

            // Breadcrumb line
            let breadcrumb = if view.breadcrumb.is_empty() {
                None
            } else {
                let path = view.breadcrumb.join(" > ");
                Some(format!("  {path} >   "))
            };

            let tags_suffix = if view.tags.is_empty() {
                String::new()
            } else {
                format!(" · {}", view.tags.join(" "))
            };
            let header_content = format!(
                "  {indicator} {name} · {state} · {} · {}{tags_suffix}  ",
                view.created_at, view.created_by
            );
            let header_width = header_content.chars().count();

            // Child lines
            let child_lines: Vec<String> = view
                .children
                .iter()
                .enumerate()
                .map(|(i, child)| {
                    let connector = if i == view.children.len() - 1 {
                        "╰─"
                    } else {
                        "├─"
                    };
                    let ci = match child.state.as_str() {
                        "wip" | "done" => "●",
                        _ => "○",
                    };
                    format!("  {connector} {ci} {}  ", child.name)
                })
                .collect();

            // Field lines
            let max_label_width = view
                .short_fields
                .iter()
                .map(|(k, _)| k.chars().count())
                .max()
                .unwrap_or(0);
            let field_lines: Vec<String> = view
                .short_fields
                .iter()
                .map(|(k, v)| {
                    let pad = max_label_width - k.chars().count();
                    format!("  {}{}: {}  ", " ".repeat(pad), k, v)
                })
                .collect();

            // Inner width
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

            // Top border
            if color {
                writeln!(out, "\x1b[2m{top}\x1b[0m").unwrap();
            } else {
                writeln!(out, "{top}").unwrap();
            }

            // Breadcrumb
            if let Some(ref bc) = breadcrumb {
                write_dim_line(&mut out, bc, bc.chars().count(), inner_width, color);
            }

            // Name line
            if color {
                let colored_tags = if view.tags.is_empty() {
                    String::new()
                } else {
                    format!(
                        "\x1b[90m ·\x1b[0m \x1b[38;5;67m{}\x1b[0m",
                        view.tags.join(" ")
                    )
                };
                let meta = format!(
                    "\x1b[90m · {} · {} · {}\x1b[0m{colored_tags}  ",
                    state, view.created_at, view.created_by
                );
                let styled_header = format!("  {}{meta}", style_yak_item(name, state, true));
                write_box_line(&mut out, &styled_header, header_width, inner_width, true);
            } else {
                write_box_line(&mut out, &header_content, header_width, inner_width, false);
            }

            // Children
            for (i, child) in view.children.iter().enumerate() {
                let connector = if i == view.children.len() - 1 {
                    "╰─"
                } else {
                    "├─"
                };
                if color {
                    let styled = format!(
                        "  {connector} {}  ",
                        style_yak_item(&child.name, &child.state, true)
                    );
                    write_box_line(
                        &mut out,
                        &styled,
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

            // Fields
            if !view.short_fields.is_empty() {
                if color {
                    writeln!(out, "\x1b[2m{divider}\x1b[0m").unwrap();
                } else {
                    writeln!(out, "{divider}").unwrap();
                }
                for line in &field_lines {
                    write_box_line(&mut out, line, line.chars().count(), inner_width, color);
                }
            }

            // Bottom
            if color {
                writeln!(out, "\x1b[2m{bottom}\x1b[0m").unwrap();
            } else {
                writeln!(out, "{bottom}").unwrap();
            }
        }

        // Context
        if view.has_context {
            self.info("");
            self.display_context(view.context.as_ref().unwrap());
        } else {
            self.info("");
            self.display_hint(&format!(
                "This yak has no context yet. Add some with:\n\n  echo \"Here's the problem...\" | yx context {}",
                view.name
            ));
        }

        // Long fields
        for (name, value) in &view.long_fields {
            self.info("");
            self.display_section_rule(name);
            let indented: String = value
                .lines()
                .map(|l| format!("  {l}"))
                .collect::<Vec<_>>()
                .join("\n");
            self.info(&indented);
        }

        self.info("");
        self.display_closing_rule();
    }

    fn show_list(&self, view: &YakTreeView) {
        if view.is_empty {
            match view.format.as_str() {
                "ids" => self.info(""), // empty output for ids
                "json" => self.info("[]"),
                "markdown" => self.info("You have no yaks. Are you done?"),
                _ => {} // pretty and plain show nothing when empty
            }
            return;
        }

        // If nodes is empty (filtered out all) and markdown format, show message
        if view.nodes.is_empty() && view.format == "markdown" {
            self.info("You have no yaks. Are you done?");
            return;
        }

        if view.format == "pretty" {
            self.info("");
        }

        self.render_tree_nodes(&view.nodes, &view.format);

        // Bottom margin only if there were nodes to display
        if view.format == "pretty" && !view.nodes.is_empty() {
            self.info("");
        }
    }

    fn show_log(&self, entries: &[LogEntryView]) {
        let mut out = self.output.lock().unwrap();
        let rule: String = "─".repeat(self.options.width);
        let color = self.options.color;

        for entry in entries {
            // Render narrative
            let rendered: String = if color {
                entry
                    .narrative
                    .iter()
                    .map(|span| {
                        if span.bold {
                            format!("\x1b[1m{}\x1b[0m", span.text)
                        } else {
                            span.text.clone()
                        }
                    })
                    .collect()
            } else {
                entry
                    .narrative
                    .iter()
                    .map(|span| span.text.clone())
                    .collect()
            };

            let sha_part = match &entry.commit_sha {
                Some(sha) if sha.len() >= 7 => format!("  sha: {}", &sha[..7]),
                Some(sha) => format!("  sha: {sha}"),
                None => String::new(),
            };

            if color {
                writeln!(out, "{rendered}").unwrap();
                writeln!(out, "\x1b[2m{}\x1b[0m", entry.relative_time).unwrap();
                writeln!(out, "\x1b[2;90mevent: {}{sha_part}\x1b[0m", entry.event_id).unwrap();
                writeln!(out, "\x1b[2m{rule}\x1b[0m").unwrap();
            } else {
                writeln!(out, "{rendered}").unwrap();
                writeln!(out, "{}", entry.relative_time).unwrap();
                writeln!(out, "event: {}{sha_part}", entry.event_id).unwrap();
                writeln!(out, "{rule}").unwrap();
            }
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
