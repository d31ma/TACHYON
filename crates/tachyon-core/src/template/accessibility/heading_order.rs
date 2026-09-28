use super::{AccessibilityProblem, ViewElement};

#[derive(Default)]
pub(super) struct HeadingOrder {
    last_level: usize,
}

impl HeadingOrder {
    const HEADINGS: [&'static str; 6] = ["h1", "h2", "h3", "h4", "h5", "h6"];
    const HELP: &'static str = "Go down one heading level at a time, as in <h2> then <h3>. Screen reader users jump between headings, and a skipped level looks like a missing part. Change the style with CSS, not with the heading level.";

    pub(super) fn problem(&mut self, element: &ViewElement) -> Option<AccessibilityProblem> {
        let level = Self::level(element.tag)?;
        let previous = std::mem::replace(&mut self.last_level, level);
        let message = format!("<h{level}> follows <h{previous}> and skips a heading level.");
        Self::skips(previous, level)
            .then(|| AccessibilityProblem::new(element.range, message, Self::HELP))
    }

    fn level(tag: &str) -> Option<usize> {
        let index = Self::HEADINGS.iter().position(|heading| *heading == tag)?;
        Some(index + 1)
    }

    fn skips(previous: usize, level: usize) -> bool {
        previous > 0 && level > previous + 1
    }
}
