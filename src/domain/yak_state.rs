use std::fmt;
use std::str::FromStr;

/// The state of a yak: Todo, Wip, Blocked, or Done.
///
/// Replaces the old `String` representation. Strings are only
/// used at serialisation boundaries (events, file storage).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YakState {
    Todo,
    Wip,
    Blocked,
    Done,
}

impl fmt::Display for YakState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YakState::Todo => write!(f, "todo"),
            YakState::Wip => write!(f, "wip"),
            YakState::Blocked => write!(f, "blocked"),
            YakState::Done => write!(f, "done"),
        }
    }
}

impl FromStr for YakState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo" => Ok(YakState::Todo),
            "wip" => Ok(YakState::Wip),
            "blocked" => Ok(YakState::Blocked),
            "done" => Ok(YakState::Done),
            _ => Err(format!(
                "Invalid state '{}'. Valid states are: todo, wip, blocked, done",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_round_trips_through_from_str() {
        for state in [
            YakState::Todo,
            YakState::Wip,
            YakState::Blocked,
            YakState::Done,
        ] {
            let s = state.to_string();
            let parsed: YakState = s.parse().unwrap();
            assert_eq!(parsed, state);
        }
    }

    #[test]
    fn from_str_rejects_invalid_state() {
        let result: Result<YakState, _> = "invalid".parse();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid state"));
        assert!(err.contains("todo, wip, blocked, done"));
    }

    #[test]
    fn display_produces_lowercase_strings() {
        assert_eq!(YakState::Todo.to_string(), "todo");
        assert_eq!(YakState::Wip.to_string(), "wip");
        assert_eq!(YakState::Blocked.to_string(), "blocked");
        assert_eq!(YakState::Done.to_string(), "done");
    }
}
