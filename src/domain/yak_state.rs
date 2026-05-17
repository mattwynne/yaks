use std::fmt;
use std::str::FromStr;

/// The stored workflow state of a yak: Todo, Wip, or Done.
///
/// `Blocked` is retained only for reading legacy data and is never valid for
/// new commands or persisted snapshots.
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
            YakState::Blocked => write!(f, "todo"),
            YakState::Done => write!(f, "done"),
        }
    }
}

impl YakState {
    pub fn from_storage(s: &str) -> Option<Self> {
        match s {
            "todo" => Some(YakState::Todo),
            "wip" => Some(YakState::Wip),
            "blocked" => Some(YakState::Blocked),
            "done" => Some(YakState::Done),
            _ => None,
        }
    }
}

impl FromStr for YakState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo" => Ok(YakState::Todo),
            "wip" => Ok(YakState::Wip),
            "done" => Ok(YakState::Done),
            _ => Err(format!(
                "Invalid state '{}'. Valid states are: todo, wip, done. Use `yx blocker add <yak> --reason ...` for external blockers.",
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
        for state in [YakState::Todo, YakState::Wip, YakState::Done] {
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
        assert!(err.contains("todo, wip, done"));
    }

    #[test]
    fn from_storage_accepts_current_and_legacy_states() {
        assert_eq!(YakState::from_storage("todo"), Some(YakState::Todo));
        assert_eq!(YakState::from_storage("wip"), Some(YakState::Wip));
        assert_eq!(YakState::from_storage("blocked"), Some(YakState::Blocked));
        assert_eq!(YakState::from_storage("done"), Some(YakState::Done));
        assert_eq!(YakState::from_storage("invalid"), None);
    }

    #[test]
    fn display_produces_lowercase_strings() {
        assert_eq!(YakState::Todo.to_string(), "todo");
        assert_eq!(YakState::Wip.to_string(), "wip");
        assert_eq!(YakState::Done.to_string(), "done");
    }
}
