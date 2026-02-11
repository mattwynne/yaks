// Event domain model - represents a logged yak operation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum YakEvent {
    Added {
        name: String,
    },

    Removed {
        name: String,
    },

    Moved {
        old_name: String,
        new_name: String,
    },

    ContextUpdated {
        name: String,
        content: String,
    },

    StateUpdated {
        name: String,
        state: String,
    },

    FieldUpdated {
        name: String,
        field_name: String,
        content: String,
    },
}

// Legacy Event struct - kept for backward compatibility during refactoring
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct Event {
    pub operation: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub author: String,
}

impl Event {
    #[allow(dead_code)]
    pub fn new(
        operation: String,
        args: Vec<String>,
        stdin: Option<String>,
        timestamp: DateTime<Utc>,
        author: String,
    ) -> Self {
        Self {
            operation,
            args,
            stdin,
            timestamp,
            author,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_added_event() {
        let event = YakEvent::Added {
            name: "test".to_string(),
        };

        match event {
            YakEvent::Added { name } => assert_eq!(name, "test"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_context_updated_event() {
        let event = YakEvent::ContextUpdated {
            name: "test".to_string(),
            content: "context".to_string(),
        };

        match event {
            YakEvent::ContextUpdated { name, content } => {
                assert_eq!(name, "test");
                assert_eq!(content, "context");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_state_updated_event() {
        let event = YakEvent::StateUpdated {
            name: "test".to_string(),
            state: "wip".to_string(),
        };

        match event {
            YakEvent::StateUpdated { name, state } => {
                assert_eq!(name, "test");
                assert_eq!(state, "wip");
            }
            _ => panic!("Wrong event type"),
        }
    }
}
