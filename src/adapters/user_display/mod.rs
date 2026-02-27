// CLI adapter - implementation using clap

use crate::domain::event_metadata::{Author, Timestamp};
use crate::domain::slug::Name;
use std::io::Write;
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
        Self::new(
            Box::new(std::io::stdout()),
            ConsoleDisplayOptions { color: true, width },
        )
    }
}

impl crate::domain::ports::DisplayPort for ConsoleDisplay {
    fn width(&self) -> usize {
        self.options.width
    }

    fn success(&self, message: &str) {
        let mut out = self.output.lock().unwrap();
        writeln!(out, "{message}").unwrap();
    }

    fn info(&self, message: &str) {
        let mut out = self.output.lock().unwrap();
        writeln!(out, "{message}").unwrap();
    }

    fn display_yak_pretty(&self, prefix: &str, name: &Name, state: &str) {
        let mut out = self.output.lock().unwrap();
        if self.options.color {
            match state {
                "wip" => writeln!(out, "{prefix}\x1b[32m●\x1b[0m \x1b[1m{name}\x1b[0m"),
                "done" => writeln!(out, "{prefix}\x1b[90m●\x1b[0m \x1b[90;9m{name}\x1b[0m"),
                _ => writeln!(out, "{prefix}○ {name}"),
            }
        } else {
            let indicator = match state {
                "wip" | "done" => "●",
                _ => "○",
            };
            writeln!(out, "{prefix}{indicator} {name}")
        }
        .unwrap();
    }

    fn display_yak_markdown(&self, depth: usize, name: &Name, state: &str) {
        let mut out = self.output.lock().unwrap();
        let indent = "  ".repeat(depth);
        let line = format!("{indent}- [{state}] {name}");
        if self.options.color && state == "done" {
            writeln!(out, "\x1b[90m{line}\x1b[0m")
        } else {
            writeln!(out, "{line}")
        }
        .unwrap();
    }

    fn display_header_box(&self, name: &Name, state: &str, created_at: &Timestamp, created_by: &Author) {
        let mut out = self.output.lock().unwrap();
        let indicator = match state {
            "wip" | "done" => "●",
            _ => "○",
        };
        let date = chrono::DateTime::from_timestamp(created_at.as_epoch_secs(), 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let content = format!("  {indicator} {name} · {state} · {date} · {}  ", created_by.name);
        let content_width = content.chars().count();
        // Use terminal width (minus 2 for │ borders), but at least content width
        let inner_width = (self.options.width.saturating_sub(2)).max(content_width);
        let padding = inner_width - content_width;
        let top = format!("┌{}┐", "─".repeat(inner_width));
        let bottom = format!("└{}┘", "─".repeat(inner_width));
        let padded_content = format!("{content}{}", " ".repeat(padding));
        let middle = format!("│{padded_content}│");

        if self.options.color {
            let meta = format!("\x1b[90m · {state} · {date} · {}  \x1b[0m", created_by.name);
            let styled_content = match state {
                "wip" => format!("  \x1b[32m●\x1b[0m \x1b[1m{name}\x1b[0m{meta}"),
                "done" => format!("  \x1b[90m●\x1b[0m \x1b[90;9m{name}\x1b[0m{meta}"),
                _ => format!("  ○ \x1b[1m{name}\x1b[0m{meta}"),
            };
            writeln!(out, "\x1b[2m{top}\x1b[0m").unwrap();
            writeln!(out, "\x1b[2m│\x1b[0m{styled_content}{}\x1b[2m│\x1b[0m", " ".repeat(padding)).unwrap();
            writeln!(out, "\x1b[2m{bottom}\x1b[0m").unwrap();
        } else {
            writeln!(out, "{top}").unwrap();
            writeln!(out, "{middle}").unwrap();
            writeln!(out, "{bottom}").unwrap();
        }
    }

    fn display_context(&self, context: &str) {
        let mut out = self.output.lock().unwrap();
        if self.options.color {
            let mut skin = termimad::MadSkin::default();
            skin.headers[0].align = termimad::Alignment::Left;
            let text = skin.term_text(context);
            write!(out, "{text}").unwrap();
        } else {
            write!(out, "{context}").unwrap();
            // Ensure trailing newline
            if !context.ends_with('\n') {
                writeln!(out).unwrap();
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
        event_id: &str,
        author_name: &str,
        author_email: &str,
        timestamp: &str,
        message: &str,
    ) {
        let mut out = self.output.lock().unwrap();
        if self.options.color {
            writeln!(out, "\x1b[33mevent {event_id}\x1b[0m").unwrap();
            writeln!(out, "Author: {author_name} <{author_email}>").unwrap();
            writeln!(out, "Date:   {timestamp}").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "    {message}").unwrap();
        } else {
            writeln!(out, "event {event_id}").unwrap();
            writeln!(out, "Author: {author_name} <{author_email}>").unwrap();
            writeln!(out, "Date:   {timestamp}").unwrap();
            writeln!(out).unwrap();
            writeln!(out, "    {message}").unwrap();
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
    use crate::domain::ports::DisplayPort;

    fn make_display(color: bool) -> (ConsoleDisplay, TestBuffer) {
        let buffer = TestBuffer::new();
        let writer = buffer.clone();
        let display = ConsoleDisplay::new(Box::new(writer), ConsoleDisplayOptions { color, width: 60 });
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
        display.display_yak_pretty("", &name, "wip");
        let output = buffer.contents();
        assert!(output.contains("\x1b["), "expected ANSI codes in: {output}");
        assert!(output.contains("my yak"));
    }

    #[test]
    fn pretty_done_with_color_has_ansi() {
        let (display, buffer) = make_display(true);
        let name = Name::from("finished yak");
        display.display_yak_pretty("", &name, "done");
        let output = buffer.contents();
        assert!(output.contains("\x1b["), "expected ANSI codes in: {output}");
    }

    #[test]
    fn pretty_wip_without_color_has_no_ansi() {
        let (display, buffer) = make_display(false);
        let name = Name::from("my yak");
        display.display_yak_pretty("", &name, "wip");
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
        display.display_yak_pretty("", &name, "done");
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
        display.display_yak_pretty("", &name, "todo");
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
        display.display_yak_markdown(0, &name, "done");
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
        display.display_yak_markdown(0, &name, "done");
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
        display.display_yak_markdown(1, &name, "todo");
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
        display.display_header_box(&name, "wip", &timestamp, &author);
        let output = buffer.contents();
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].starts_with('┌'), "Expected top border, got: {:?}", lines[0]);
        assert!(lines[0].ends_with('┐'), "Expected top border end, got: {:?}", lines[0]);
        assert!(lines[1].contains("● my yak"), "Expected name in box, got: {:?}", lines[1]);
        assert!(lines[1].contains("wip"), "Expected state in box, got: {:?}", lines[1]);
        assert!(lines[1].contains("2025-02-19"), "Expected date in box, got: {:?}", lines[1]);
        assert!(lines[1].contains("Matt Wynne"), "Expected author in box, got: {:?}", lines[1]);
        assert!(lines[2].starts_with('└'), "Expected bottom border, got: {:?}", lines[2]);
        assert!(lines[2].ends_with('┘'), "Expected bottom border end, got: {:?}", lines[2]);
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
        assert!(output.contains("\x1b[2m"), "expected dim ANSI code in: {output}");
        assert!(output.contains("root > mid > "));
        assert!(output.contains("\x1b[0m"), "expected reset ANSI code in: {output}");
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
}

/// Create a `ConsoleDisplay` + `TestBuffer` pair for use in tests.
/// Output is plain text (no ANSI color codes).
#[cfg(any(test, feature = "test-support"))]
pub fn make_test_display() -> (ConsoleDisplay, TestBuffer) {
    let buffer = TestBuffer::new();
    let writer = buffer.clone();
    let display = ConsoleDisplay::new(Box::new(writer), ConsoleDisplayOptions { color: false, width: 60 });
    (display, buffer)
}
