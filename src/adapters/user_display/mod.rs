// CLI adapter - implementation using clap

#[cfg(any(test, feature = "test-support"))]
pub mod memory_display;

use crate::domain::slug::Name;

pub struct ConsoleDisplay;

impl crate::domain::ports::DisplayPort for ConsoleDisplay {
    fn success(&self, message: &str) {
        println!("{message}");
    }

    fn info(&self, message: &str) {
        println!("{message}");
    }

    fn display_yak_pretty(&self, prefix: &str, name: &Name, state: &str) {
        match state {
            "wip" => println!("{prefix}\x1b[32m●\x1b[0m \x1b[1m{name}\x1b[0m"),
            "done" => println!("{prefix}\x1b[90m●\x1b[0m \x1b[90;9m{name}\x1b[0m"),
            _ => println!("{prefix}○ {name}"),
        }
    }

    fn display_yak_markdown(&self, depth: usize, name: &Name, state: &str) {
        let indent = "  ".repeat(depth);
        let line = format!("{indent}- [{state}] {name}");
        if state == "done" {
            println!("\x1b[90m{line}\x1b[0m");
        } else {
            println!("{line}");
        }
    }

    fn log_entry(&self, author_name: &str, author_email: &str, timestamp: &str, message: &str) {
        println!("{} <{}>  {}", author_name, author_email, timestamp);
        println!("{}", message);
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use memory_display::InMemoryDisplay;
