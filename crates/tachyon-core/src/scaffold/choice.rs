pub trait Choice: Copy + Eq + 'static {
    const ALL: &'static [Self];
    const QUESTION: &'static str;

    fn names(self) -> &'static str;

    fn label(self) -> &'static str;

    fn flag_name(self) -> &'static str {
        self.names().split(',').next().unwrap_or_default()
    }

    fn named(text: &str) -> Option<Self> {
        let wanted = text.trim().to_ascii_lowercase();
        Self::ALL
            .iter()
            .copied()
            .find(|choice| choice.names().split(',').any(|name| name == wanted))
    }

    fn flag_names() -> String {
        let names: Vec<&str> = Self::ALL.iter().map(|choice| choice.flag_name()).collect();
        names.join(", ")
    }
}
