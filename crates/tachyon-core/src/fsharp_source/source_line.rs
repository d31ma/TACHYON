pub(crate) struct SourceLine<'a> {
    pub(crate) text: &'a str,
    pub(crate) code: &'a [bool],
}

impl<'a> SourceLine<'a> {
    pub(crate) fn split(text: &'a str, code: &'a [bool]) -> Vec<Self> {
        Self::lines(text, code)
    }

    fn lines(text: &'a str, code: &'a [bool]) -> Vec<Self> {
        let mut lines = Vec::new();
        let mut start = 0;
        for line in text.split('\n') {
            let end = start + line.len();
            lines.push(Self {
                text: line.trim_end_matches('\r'),
                code: &code[start..end],
            });
            start = end + 1;
        }
        lines
    }

    pub(crate) fn indent(&self) -> usize {
        self.text.len() - self.text.trim_start().len()
    }

    pub(crate) fn trimmed(&self) -> &'a str {
        self.text.trim()
    }

    pub(crate) fn trimmed_code(&self) -> &'a [bool] {
        let end = self.indent() + self.trimmed().len();
        &self.code[self.indent()..end]
    }
}

#[cfg(test)]
mod tests {
    use super::SourceLine;

    #[test]
    fn lines_know_their_start_and_indent() {
        let text = "type A() =\r\n    member x.B = 1\n";
        let code = vec![true; text.len()];
        let lines = SourceLine::split(text, &code);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1].indent(), 4);
        assert_eq!(lines[1].trimmed(), "member x.B = 1");
        assert_eq!(lines[0].text, "type A() =");
    }
}
