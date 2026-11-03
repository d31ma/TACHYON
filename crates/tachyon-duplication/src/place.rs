#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Place {
    pub path: String,
    pub first_line: usize,
    pub last_line: usize,
}

impl Place {
    #[must_use]
    pub fn lines(&self) -> usize {
        self.last_line.saturating_sub(self.first_line) + 1
    }

    #[must_use]
    pub fn label(&self) -> String {
        format!("{}:{}-{}", self.path, self.first_line, self.last_line)
    }
}
