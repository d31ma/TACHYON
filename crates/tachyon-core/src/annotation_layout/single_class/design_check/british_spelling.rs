#![allow(clippy::needless_bitwise_bool)]

use super::english_words::EnglishWords;

pub(super) struct BritishSpelling;

impl BritishSpelling {
    const AMERICAN_FORMS: [&'static str; 15] = [
        "our>or",
        "ise>ize",
        "isi>izi",
        "isa>iza",
        "yse>yze",
        "tre>ter",
        "bre>ber",
        "ogue>og",
        "ence>ense",
        "elled>eled",
        "elling>eling",
        "eller>eler",
        "grey>gray",
        "gement>gment",
        "mme>m",
    ];
    const ARROW: char = '>';

    pub(super) fn is_english(word: &str) -> bool {
        let forms: Vec<(&str, &str)> = Self::AMERICAN_FORMS
            .iter()
            .filter_map(|form| form.split_once(Self::ARROW))
            .collect();
        let combined = forms
            .iter()
            .fold(word.to_owned(), |form, (british, american)| {
                form.replace(british, american)
            });
        let single = forms
            .iter()
            .any(|(british, american)| EnglishWords::contains(&word.replace(british, american)));
        single | EnglishWords::contains(&combined)
    }
}
