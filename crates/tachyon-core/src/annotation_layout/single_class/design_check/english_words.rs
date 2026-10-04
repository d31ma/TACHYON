use std::collections::HashSet;
use std::io::Read as _;
use std::sync::OnceLock;

pub(super) struct EnglishWords;

impl EnglishWords {
    pub(super) const COMPRESSED: &'static [u8] = include_bytes!("english_words.txt.br");
    const BUFFER_BYTES: usize = 4096;

    pub(super) fn contains(word: &str) -> bool {
        Self::words().contains(word)
    }

    pub(super) fn decompressed() -> String {
        let mut text = String::new();
        let mut reader = brotli::Decompressor::new(Self::COMPRESSED, Self::BUFFER_BYTES);
        match reader.read_to_string(&mut text) {
            Ok(_) => text,
            Err(_) => String::new(),
        }
    }

    fn words() -> &'static HashSet<&'static str> {
        static WORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
        WORDS.get_or_init(|| {
            let text: &'static str = Box::leak(Self::decompressed().into_boxed_str());
            text.lines().collect()
        })
    }
}
