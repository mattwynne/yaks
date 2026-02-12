use anyhow::Result;

/// Trait for serializing/deserializing individual event types
pub trait EventFormat {
    /// Tag name for this event (e.g., "Added", "StateUpdated")
    fn event_tag(&self) -> &'static str;
    /// Serialize event data (everything after "Tag: ")
    fn format_data(&self) -> String;
    /// Deserialize event data from string
    fn parse_data(data: &str) -> Result<Self>
    where
        Self: Sized;
}

/// Parse space-separated quoted values: `"foo" "bar"` -> `["foo", "bar"]`
pub fn parse_quoted_values(data: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    let mut chars = data.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek() == Some(&' ') {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        // Expect opening quote
        if chars.next() != Some('"') {
            anyhow::bail!("Expected '\"' in event data: {}", data);
        }
        // Read until closing quote
        let mut value = String::new();
        loop {
            match chars.next() {
                Some('"') => break,
                Some(c) => value.push(c),
                None => anyhow::bail!("Unterminated quote in event data: {}", data),
            }
        }
        values.push(value);
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_quoted_value() {
        let values = parse_quoted_values("\"foo\"").unwrap();
        assert_eq!(values, vec!["foo"]);
    }

    #[test]
    fn parses_multiple_quoted_values() {
        let values = parse_quoted_values("\"foo\" \"bar\"").unwrap();
        assert_eq!(values, vec!["foo", "bar"]);
    }

    #[test]
    fn parses_values_with_spaces() {
        let values = parse_quoted_values("\"foo bar\" \"baz\"").unwrap();
        assert_eq!(values, vec!["foo bar", "baz"]);
    }

    #[test]
    fn errors_on_missing_quote() {
        assert!(parse_quoted_values("foo").is_err());
    }

    #[test]
    fn errors_on_unterminated_quote() {
        assert!(parse_quoted_values("\"foo").is_err());
    }
}
