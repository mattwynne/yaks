use crate::domain::{Yak, YakState};
use crate::ports::{OutputFormat, OutputFormatter, YakFilter};
use std::cmp::Ordering;

pub struct TerminalFormatter;

impl Default for TerminalFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalFormatter {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn sort_yaks(yaks: &mut [Yak]) {
        yaks.sort_by(|a, b| {
            let a_parent = a.parent().unwrap_or_else(|| ".".to_string());
            let b_parent = b.parent().unwrap_or_else(|| ".".to_string());

            if a_parent != b_parent {
                return a.name.cmp(&b.name);
            }

            match (a.state, b.state) {
                (YakState::Done, YakState::Todo) => Ordering::Less,
                (YakState::Todo, YakState::Done) => Ordering::Greater,
                _ => a.mtime.cmp(&b.mtime),
            }
        });
    }

    fn format_markdown_yak(yak: &Yak) -> String {
        let depth = yak.depth();
        let indent = "  ".repeat(depth);
        let display_name = yak.basename();

        match yak.state {
            YakState::Done => format!("\x1b[90m{indent}- [x] {display_name}\x1b[0m"),
            YakState::Todo => format!("{indent}- [ ] {display_name}"),
        }
    }

    fn format_plain_yak(yak: &Yak) -> String {
        yak.name.clone()
    }

    fn should_display(yak: &Yak, filter: &YakFilter) -> bool {
        match filter {
            YakFilter::All => true,
            YakFilter::NotDone => yak.state != YakState::Done,
            YakFilter::Done => yak.state == YakState::Done,
        }
    }
}

impl OutputFormatter for TerminalFormatter {
    fn format_yak_list(&self, yaks: &[Yak], format: OutputFormat, filter: YakFilter) -> String {
        if yaks.is_empty() {
            return self.format_empty_list(format);
        }

        let mut sorted_yaks: Vec<Yak> = yaks.to_vec();
        Self::sort_yaks(&mut sorted_yaks);

        let filtered: Vec<String> = sorted_yaks
            .iter()
            .filter(|y| Self::should_display(y, &filter))
            .map(|y| match format {
                OutputFormat::Markdown => Self::format_markdown_yak(y),
                OutputFormat::Plain => Self::format_plain_yak(y),
            })
            .collect();

        filtered.join("\n")
    }

    fn format_yak_with_context(&self, yak: &Yak) -> String {
        if yak.context.is_empty() {
            yak.name.clone()
        } else {
            format!("{}\n\n{}", yak.name, yak.context)
        }
    }

    fn format_empty_list(&self, format: OutputFormat) -> String {
        match format {
            OutputFormat::Markdown => "You have no yaks. Are you done?".to_string(),
            OutputFormat::Plain => String::new(),
        }
    }
}
