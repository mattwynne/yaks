use crate::domain::YakEvent;
#[allow(unused_imports)]
use crate::ports::Store;
#[allow(unused_imports)]
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YakState {
    pub(crate) state: String,
    pub(crate) context: Option<String>,
}

pub struct YakMap {
    yaks: HashMap<String, YakState>,
    pending_events: Vec<YakEvent>,
}

impl YakMap {
    pub fn new() -> Self {
        Self {
            yaks: HashMap::new(),
            pending_events: Vec::new(),
        }
    }

    pub fn take_events(&mut self) -> Vec<YakEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_yak_map_is_empty() {
        let map = YakMap::new();
        assert_eq!(map.yaks.len(), 0);
        assert_eq!(map.pending_events.len(), 0);
    }

    #[test]
    fn test_take_events_removes_events() {
        let mut map = YakMap::new();
        map.pending_events.push(YakEvent::Added {
            name: "test".to_string()
        });

        let events = map.take_events();

        assert_eq!(events.len(), 1);
        assert_eq!(map.pending_events.len(), 0);
    }
}
