pub(crate) struct CppTypeText;

impl CppTypeText {
    const QUALIFIERS: [&'static str; 3] = ["const", "volatile", "typename"];

    pub(crate) fn plain(type_text: &str) -> String {
        let words: Vec<&str> = type_text
            .split_whitespace()
            .filter(|word| !Self::QUALIFIERS.contains(word))
            .collect();
        words
            .join(" ")
            .trim_end_matches(['&', '*', ' '])
            .trim()
            .to_owned()
    }
}
