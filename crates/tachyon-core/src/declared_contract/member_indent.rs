pub(crate) struct MemberIndent<'a> {
    pub(crate) indent: &'a str,
}

impl MemberIndent<'_> {
    pub(crate) fn block(&self, text: &str) -> String {
        let lines: Vec<String> = text.lines().map(|line| self.line(line)).collect();
        format!("\n{}\n", lines.join("\n"))
    }

    fn line(&self, line: &str) -> String {
        let indent = &self.indent[..self.indent.len() * usize::from(!line.is_empty())];
        format!("{indent}{line}")
    }
}
