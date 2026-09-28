#![allow(clippy::needless_bitwise_bool)]

use super::british_spelling::BritishSpelling;
use super::english_words::EnglishWords;
use super::short_words::ShortWords;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WordCheck {
    Plain,
    Clipped(&'static str),
    Unknown,
}

impl WordCheck {
    const SHORTEST_FIRST_HALF: usize = 3;
    const SHORTEST_SECOND_HALF: usize = 2;
    const LONGEST_WORD: usize = 32;

    pub(super) fn of(part: &str, project: &[String]) -> Self {
        let clipped = ShortWords::full_word(part).map(Self::Clipped);
        let plain = Self::is_readable_and_plain(part, project).then_some(Self::Plain);
        clipped.or(plain).unwrap_or(Self::Unknown)
    }

    fn is_readable_and_plain(part: &str, project: &[String]) -> bool {
        part.len() <= Self::LONGEST_WORD && Self::is_plain(part, project)
    }

    fn is_plain(part: &str, project: &[String]) -> bool {
        let listed = project.iter().any(|word| word == part);
        let single = part.trim_end_matches('s');
        let project_plural = project.iter().any(|word| word == single);
        let english = EnglishWords::contains(part)
            | ShortWords::is_technical(part)
            | BritishSpelling::is_english(part);
        listed | project_plural | english | Self::is_compound(part)
    }

    fn is_compound(part: &str) -> bool {
        let last = part.len().min(Self::LONGEST_WORD);
        (Self::SHORTEST_FIRST_HALF..last)
            .filter(|&at| part.is_char_boundary(at))
            .any(|at| Self::is_word_pair(&part[..at], &part[at..]))
    }

    fn is_word_pair(first: &str, second: &str) -> bool {
        let long_enough = second.len() >= Self::SHORTEST_SECOND_HALF;
        let clipped =
            ShortWords::full_word(first).is_some() | ShortWords::full_word(second).is_some();
        let english = EnglishWords::contains(first) & EnglishWords::contains(second);
        long_enough & !clipped & english
    }
}
