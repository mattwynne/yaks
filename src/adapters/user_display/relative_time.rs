use chrono::DateTime;
use chrono_humanize::HumanTime;

/// Format epoch seconds as a human-readable relative time string (e.g. "3 hours ago").
///
/// Returns `"unknown"` if the timestamp cannot be converted to a valid datetime.
pub fn format_relative(epoch_secs: i64) -> String {
    match DateTime::from_timestamp(epoch_secs, 0) {
        Some(dt) => {
            let human = HumanTime::from(dt);
            human.to_string()
        }
        None => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_timestamp_contains_ago_or_now() {
        let now = chrono::Utc::now().timestamp();
        let result = format_relative(now - 5);
        assert!(
            result.contains("ago") || result.contains("now"),
            "Expected relative time to contain 'ago' or 'now', got: {result}"
        );
    }

    #[test]
    fn old_timestamp_contains_ago() {
        let one_day_ago = chrono::Utc::now().timestamp() - 86400;
        let result = format_relative(one_day_ago);
        assert!(
            result.contains("ago"),
            "Expected relative time to contain 'ago', got: {result}"
        );
    }

    #[test]
    fn invalid_timestamp_returns_unknown() {
        let result = format_relative(i64::MIN);
        assert_eq!(result, "unknown");
    }
}
