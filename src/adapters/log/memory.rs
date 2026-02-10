// In-memory log adapter - for testing only

use crate::ports::LogPort;
use anyhow::Result;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
#[allow(dead_code)]
pub struct InMemoryLog {
    // Vec of logged commands
    commands: Arc<RwLock<Vec<String>>>,
}

#[allow(dead_code)]
impl InMemoryLog {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all logged commands (for testing/inspection)
    #[allow(dead_code)]
    pub fn get_commands(&self) -> Vec<String> {
        self.commands.read().unwrap().clone()
    }

    /// Clear all logged commands (for testing)
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.commands.write().unwrap().clear();
    }
}

impl Default for InMemoryLog {
    fn default() -> Self {
        Self::new()
    }
}

impl LogPort for InMemoryLog {
    fn log_command(&self, command: &str) -> Result<()> {
        self.commands.write().unwrap().push(command.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_command() {
        let log = InMemoryLog::new();
        log.log_command("add test-yak").unwrap();

        let commands = log.get_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], "add test-yak");
    }

    #[test]
    fn test_log_multiple_commands() {
        let log = InMemoryLog::new();
        log.log_command("add yak1").unwrap();
        log.log_command("add yak2").unwrap();
        log.log_command("done yak1").unwrap();

        let commands = log.get_commands();
        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0], "add yak1");
        assert_eq!(commands[1], "add yak2");
        assert_eq!(commands[2], "done yak1");
    }

    #[test]
    fn test_clear() {
        let log = InMemoryLog::new();
        log.log_command("add test-yak").unwrap();
        log.clear();

        let commands = log.get_commands();
        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let log = InMemoryLog::new();
        let mut handles = vec![];

        // Spawn multiple threads that log commands
        for i in 0..10 {
            let log_clone = log.clone();
            let handle = thread::spawn(move || {
                log_clone.log_command(&format!("command{}", i)).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all commands were logged
        let commands = log.get_commands();
        assert_eq!(commands.len(), 10);
    }
}
