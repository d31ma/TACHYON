#![allow(clippy::expect_used)]

use super::english_words::EnglishWords;
use brotli::enc::BrotliEncoderParams;

struct WordList;

impl WordList {
    const TEXT: &'static str = include_str!("english_words.txt");
    const BROTLI_QUALITY: i32 = 11;
    const BROTLI_WINDOW_BITS: i32 = 24;
    const COMPRESSED_PATH: &'static str =
        "src/annotation_layout/single_class/design_check/english_words.txt.br";

    fn compressed() -> Vec<u8> {
        let params = BrotliEncoderParams {
            quality: Self::BROTLI_QUALITY,
            lgwin: Self::BROTLI_WINDOW_BITS,
            size_hint: Self::TEXT.len(),
            ..BrotliEncoderParams::default()
        };
        let mut encoded = Vec::new();
        brotli::BrotliCompress(&mut Self::TEXT.as_bytes(), &mut encoded, &params)
            .expect("compressed word list");
        encoded
    }
}

#[test]
fn the_compressed_list_holds_every_word_of_the_text_list() {
    assert!(
        EnglishWords::decompressed() == WordList::TEXT,
        "english_words.txt.br is out of date; run: cargo test -p tachyon-core --lib english_words_tests -- --ignored"
    );
    assert!(EnglishWords::contains("color"));
    assert!(EnglishWords::COMPRESSED.len() * 3 < WordList::TEXT.len());
}

#[test]
#[ignore = "rewrites english_words.txt.br after an edit of english_words.txt"]
fn write_the_compressed_list() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(WordList::COMPRESSED_PATH);
    std::fs::write(path, WordList::compressed()).expect("written word list");
}
