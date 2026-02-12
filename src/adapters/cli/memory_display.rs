// In-memory display adapter - for testing only

use crate::ports::DisplayPort;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct InMemoryDisplay {
    success_messages: Arc<RwLock<Vec<String>>>,
    info_messages: Arc<RwLock<Vec<String>>>,
}

impl InMemoryDisplay {
    pub fn new() -> Self {
        Self {
            success_messages: Arc::new(RwLock::new(Vec::new())),
            info_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn get_success_messages(&self) -> Vec<String> {
        self.success_messages.read().unwrap().clone()
    }

    pub fn get_info_messages(&self) -> Vec<String> {
        self.info_messages.read().unwrap().clone()
    }

    pub fn get_all_messages(&self) -> Vec<String> {
        let mut all = Vec::new();
        all.extend(self.success_messages.read().unwrap().clone());
        all.extend(self.info_messages.read().unwrap().clone());
        all
    }

    pub fn clear(&self) {
        self.success_messages.write().unwrap().clear();
        self.info_messages.write().unwrap().clear();
    }
}

impl Default for InMemoryDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayPort for InMemoryDisplay {
    fn success(&self, message: &str) {
        self.success_messages
            .write()
            .unwrap()
            .push(message.to_string());
    }

    fn info(&self, message: &str) {
        self.info_messages
            .write()
            .unwrap()
            .push(message.to_string());
    }

    fn display_yak_pretty(&self, prefix: &str, name: &str, state: &str) {
        let indicator = match state {
            "wip" => "●",
            "done" => "●",
            _ => "○",
        };
        self.info_messages
            .write()
            .unwrap()
            .push(format!("{prefix}{indicator} {name}"));
    }

    fn display_yak_markdown(&self, depth: usize, name: &str, state: &str) {
        let indent = "  ".repeat(depth);
        self.info_messages
            .write()
            .unwrap()
            .push(format!("{indent}- [{state}] {name}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_message() {
        let output = InMemoryDisplay::new();
        output.success("Operation successful");

        let messages = output.get_success_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "Operation successful");
    }

    #[test]
    fn test_info_message() {
        let output = InMemoryDisplay::new();
        output.info("Information");

        let messages = output.get_info_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "Information");
    }

    #[test]
    fn test_multiple_messages() {
        let output = InMemoryDisplay::new();
        output.success("Success 1");
        output.info("Info 1");
        output.success("Success 2");

        assert_eq!(output.get_success_messages().len(), 2);
        assert_eq!(output.get_info_messages().len(), 1);
        assert_eq!(output.get_all_messages().len(), 3);
    }

    #[test]
    fn test_clear() {
        let output = InMemoryDisplay::new();
        output.success("Success");
        output.info("Info");

        output.clear();

        assert_eq!(output.get_success_messages().len(), 0);
        assert_eq!(output.get_info_messages().len(), 0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let output = InMemoryDisplay::new();
        let mut handles = vec![];

        for i in 0..10 {
            let output_clone = output.clone();
            let handle = thread::spawn(move || {
                output_clone.success(&format!("success{}", i));
                output_clone.info(&format!("info{}", i));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(output.get_success_messages().len(), 10);
        assert_eq!(output.get_info_messages().len(), 10);
    }
}
