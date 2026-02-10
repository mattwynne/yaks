// CLI adapter - implementation using clap

pub mod memory;

pub struct ConsoleOutput;

impl crate::ports::OutputPort for ConsoleOutput {
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

pub use memory::InMemoryOutput;
