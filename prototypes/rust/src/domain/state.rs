use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YakState {
    Todo,
    Done,
}

impl FromStr for YakState {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim() {
            "done" => YakState::Done,
            _ => YakState::Todo,
        })
    }
}

impl YakState {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            YakState::Todo => "todo",
            YakState::Done => "done",
        }
    }
}

impl fmt::Display for YakState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
