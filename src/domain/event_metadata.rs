#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub email: String,
}

impl Author {
    pub fn unknown() -> Self {
        Self {
            name: "unknown".to_string(),
            email: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp(pub i64);

impl Timestamp {
    pub fn now() -> Self {
        Self(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        )
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn as_epoch_secs(&self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMetadata {
    pub author: Author,
    pub timestamp: Timestamp,
    pub event_id: Option<String>,
}

impl EventMetadata {
    pub fn new(author: Author, timestamp: Timestamp) -> Self {
        Self {
            author,
            timestamp,
            event_id: None,
        }
    }

    pub fn default_legacy() -> Self {
        Self {
            author: Author::unknown(),
            timestamp: Timestamp::zero(),
            event_id: None,
        }
    }
}
