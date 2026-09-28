use std::collections::HashSet;
use std::sync::OnceLock;

pub(super) struct EnglishWords;

impl EnglishWords {
    pub(super) fn contains(word: &str) -> bool {
        Self::words().contains(word)
    }

    fn words() -> &'static HashSet<&'static str> {
        static WORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
        WORDS.get_or_init(|| include_str!("english_words.txt").lines().collect())
    }
}
