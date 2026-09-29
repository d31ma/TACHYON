#[derive(Debug, Default, Clone)]
pub(super) struct ShadowLines {
    lines: Vec<String>,
    sources: Vec<usize>,
}

impl ShadowLines {
    pub(super) fn push(&mut self, text: impl Into<String>, source_line: usize) {
        self.lines.push(text.into());
        self.sources.push(source_line);
    }

    pub(super) fn text(&self) -> String {
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }

    pub(super) fn source_line(&self, shadow_line: usize) -> Option<usize> {
        self.sources.get(shadow_line).copied()
    }
}
