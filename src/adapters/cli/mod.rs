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
}

pub use memory_display::InMemoryDisplay;
