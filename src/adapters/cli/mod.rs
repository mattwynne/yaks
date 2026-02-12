// CLI adapter - implementation using clap

pub mod memory_display;

pub struct ConsoleDisplay;

impl crate::ports::DisplayPort for ConsoleDisplay {
    fn success(&self, message: &str) {
        println!("{message}");
    }

    fn error(&self, message: &str) {
        eprintln!("Error: {message}");
    }

    fn info(&self, message: &str) {
        println!("{message}");
    }

    fn display_yak_pretty(&self, prefix: &str, name: &str, state: &str) {
        match state {
            "wip" => println!("{prefix}\x1b[32m●\x1b[0m \x1b[1m{name}\x1b[0m"),
            "done" => println!("{prefix}\x1b[90m●\x1b[0m \x1b[90;9m{name}\x1b[0m"),
            _ => println!("{prefix}○ {name}"),
        }
    }

    fn display_yak_markdown(&self, depth: usize, name: &str, state: &str) {
        let indent = "  ".repeat(depth);
        let line = format!("{indent}- [{state}] {name}");
        if state == "done" {
            println!("\x1b[90m{line}\x1b[0m");
        } else {
            println!("{line}");
        }
    }
}

pub use memory_display::InMemoryDisplay;
