// In-memory display adapter - for testing only

use crate::ports::DisplayPort;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
#[allow(dead_code)]
pub struct InMemoryDisplay {
    // Separate buffers for each output type
    success_messages: Arc<RwLock<Vec<String>>>,
    error_messages: Arc<RwLock<Vec<String>>>,
    info_messages: Arc<RwLock<Vec<String>>>,
}

#[allow(dead_code)]
impl InMemoryDisplay {
    pub fn new() -> Self {
        Self {
            success_messages: Arc::new(RwLock::new(Vec::new())),
            error_messages: Arc::new(RwLock::new(Vec::new())),
            info_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all success messages (for testing/inspection)
    #[allow(dead_code)]
    pub fn get_success_messages(&self) -> Vec<String> {
        self.success_messages.read().unwrap().clone()
    }

    /// Get all error messages (for testing/inspection)
    #[allow(dead_code)]
    pub fn get_error_messages(&self) -> Vec<String> {
        self.error_messages.read().unwrap().clone()
    }

    /// Get all info messages (for testing/inspection)
    #[allow(dead_code)]
    pub fn get_info_messages(&self) -> Vec<String> {
        self.info_messages.read().unwrap().clone()
    }

    /// Get all messages combined (for testing)
    #[allow(dead_code)]
    pub fn get_all_messages(&self) -> Vec<String> {
        let mut all = Vec::new();
        all.extend(self.success_messages.read().unwrap().clone());
        all.extend(self.error_messages.read().unwrap().clone());
        all.extend(self.info_messages.read().unwrap().clone());
        all
    }

    /// Clear all messages (for testing)
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.success_messages.write().unwrap().clear();
        self.error_messages.write().unwrap().clear();
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

    fn error(&self, message: &str) {
        self.error_messages
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
    fn test_error_message() {
        let output = InMemoryDisplay::new();
        output.error("Something went wrong");

        let messages = output.get_error_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "Something went wrong");
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
        output.error("Error 1");
        output.info("Info 1");
        output.success("Success 2");

        assert_eq!(output.get_success_messages().len(), 2);
        assert_eq!(output.get_error_messages().len(), 1);
        assert_eq!(output.get_info_messages().len(), 1);
        assert_eq!(output.get_all_messages().len(), 4);
    }

    #[test]
    fn test_clear() {
        let output = InMemoryDisplay::new();
        output.success("Success");
        output.error("Error");
        output.info("Info");

        output.clear();

        assert_eq!(output.get_success_messages().len(), 0);
        assert_eq!(output.get_error_messages().len(), 0);
        assert_eq!(output.get_info_messages().len(), 0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let output = InMemoryDisplay::new();
        let mut handles = vec![];

        // Spawn multiple threads that write messages
        for i in 0..10 {
            let output_clone = output.clone();
            let handle = thread::spawn(move || {
                output_clone.success(&format!("success{}", i));
                output_clone.error(&format!("error{}", i));
                output_clone.info(&format!("info{}", i));
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all messages were captured
        assert_eq!(output.get_success_messages().len(), 10);
        assert_eq!(output.get_error_messages().len(), 10);
        assert_eq!(output.get_info_messages().len(), 10);
    }
}
