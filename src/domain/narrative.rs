/// A span of text in a narrative, either plain or highlighted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NarrativeSpan {
    Plain(String),
    Highlight(String),
}

/// A narrative is a sequence of spans.
pub type Narrative = Vec<NarrativeSpan>;

/// Helper to build a plain text span.
pub fn plain(s: &str) -> NarrativeSpan {
    NarrativeSpan::Plain(s.to_string())
}

/// Helper to build a highlighted text span.
pub fn highlight(s: &str) -> NarrativeSpan {
    NarrativeSpan::Highlight(s.to_string())
}

/// Convert a narrative to plain text (for tests and no-color mode).
pub fn to_plain_text(narrative: &[NarrativeSpan]) -> String {
    narrative
        .iter()
        .map(|s| match s {
            NarrativeSpan::Plain(t) | NarrativeSpan::Highlight(t) => t.as_str(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_creates_plain_span() {
        assert_eq!(plain("hello"), NarrativeSpan::Plain("hello".to_string()));
    }

    #[test]
    fn highlight_creates_highlight_span() {
        assert_eq!(
            highlight("world"),
            NarrativeSpan::Highlight("world".to_string())
        );
    }

    #[test]
    fn to_plain_text_concatenates_all_spans() {
        let spans = vec![highlight("Matt"), plain(" added "), highlight("my yak")];
        assert_eq!(to_plain_text(&spans), "Matt added my yak");
    }

    #[test]
    fn to_plain_text_empty() {
        let spans: Vec<NarrativeSpan> = vec![];
        assert_eq!(to_plain_text(&spans), "");
    }
}
