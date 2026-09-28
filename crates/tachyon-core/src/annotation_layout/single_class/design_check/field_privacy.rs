#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;

pub(super) struct FieldPrivacy;

impl FieldPrivacy {
    const PRIVATE_WORDS: [&'static str; 3] = ["private", "fileprivate", "protected"];
    const RUBY_SECTION: &'static str = "private";
    const PUB: &'static str = "pub";
    const HASH: &'static str = "#";
    const UNDERSCORE: char = '_';

    pub(super) fn is_private(class: &ClassFile, at: usize) -> bool {
        Self::private_in_language(class, at)
    }

    fn private_in_language(class: &ClassFile, at: usize) -> bool {
        let name = class.words.text(at);
        match class.extension {
            "js" => {
                (class.words.text(at.wrapping_sub(1)) == Self::HASH)
                    | name.starts_with(Self::UNDERSCORE)
            }
            "dart" | "py" => name.starts_with(Self::UNDERSCORE),
            "go" => name.starts_with(char::is_lowercase),
            "rs" => !Self::has_word_before(class, at, &[Self::PUB]),
            "rb" => Self::in_ruby_private_part(class, at),
            _ => Self::has_word_before(class, at, &Self::PRIVATE_WORDS),
        }
    }

    fn has_word_before(class: &ClassFile, at: usize, wanted: &[&str]) -> bool {
        let line = class.index.line_begin(at)..at;
        class.words.has_word(line, wanted)
    }

    fn in_ruby_private_part(class: &ClassFile, at: usize) -> bool {
        (0..at).any(|index| {
            let word = class.words.text(index) == Self::RUBY_SECTION;
            let opens_line = class.words.line_start(index);
            let alone = class.words.line_start(index + 1);
            let marks_line = class.index.line_begin(at) == index;
            word & opens_line & (alone | marks_line)
        })
    }
}
