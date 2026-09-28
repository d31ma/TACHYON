#![allow(clippy::needless_bitwise_bool)]

use super::edits::TextEdit;
use super::scan::SourceText;
use crate::annotation_layout::UnusedMember;

pub(super) struct PrivateKeyword;

impl PrivateKeyword {
    const PRIVATE: &'static str = "private";
    const PRIVATE_SPACED: &'static str = "private ";
    const PUBLIC_WORDS: [&'static str; 2] = ["public", "internal"];
    const PUB: &'static str = "pub";
    const RUBY: &'static str = "rb";
    const RUST: &'static str = "rs";
    const RUBY_HEADS: [&'static str; 4] = ["def", "attr_reader", "attr_accessor", "attr_writer"];
    const RUBY_CLASS_METHOD: &'static str = "self.";
    const MARKS: [char; 3] = ['@', '[', '#'];

    pub(super) fn edits(
        text: &SourceText<'_>,
        language: &str,
        member: &UnusedMember,
    ) -> Option<Vec<TextEdit>> {
        let line = text.line_start(member.start);
        let head = &text.source[line..member.start];
        let edit = Self::edit(language, line, head, text.line_text(line))?;
        Some(vec![edit])
    }

    fn edit(language: &str, line: usize, head: &str, line_text: &str) -> Option<TextEdit> {
        match language {
            Self::RUST => Self::without_pub(line, head),
            Self::RUBY => Self::ruby_private(line, head, line_text),
            _ => Self::keyword_private(line, head),
        }
    }

    fn without_pub(line: usize, head: &str) -> Option<TextEdit> {
        let start = Self::word_at(head, Self::PUB)?;
        let rest = &head[start + Self::PUB.len()..];
        let scope = rest
            .strip_prefix('(')
            .and_then(|inner| inner.find(')'))
            .map_or(0, |close| close + 2);
        let after = &rest[scope..];
        let spaces = after.len() - after.trim_start().len();
        let end = start + Self::PUB.len() + scope + spaces;
        Some(TextEdit::delete(line + start, line + end))
    }

    fn ruby_private(line: usize, head: &str, line_text: &str) -> Option<TextEdit> {
        let written = head.trim_start();
        let first = written.split_whitespace().next().unwrap_or("");
        let plain = Self::RUBY_HEADS.contains(&first);
        let single = !line_text.contains(',') & !line_text.contains(Self::RUBY_CLASS_METHOD);
        let indent = head.len() - written.len();
        (plain & single).then(|| TextEdit::insert(line + indent, Self::PRIVATE_SPACED))
    }

    fn keyword_private(line: usize, head: &str) -> Option<TextEdit> {
        let written = head.trim_start();
        let marked = written.starts_with(Self::MARKS);
        let indent = head.len() - written.len();
        let public = Self::PUBLIC_WORDS
            .iter()
            .find_map(|word| Self::word_at(head, word).map(|at| (at, word.len())));
        let edit = public.map_or_else(
            || TextEdit::insert(line + indent, Self::PRIVATE_SPACED),
            |(at, length)| TextEdit::replace(line + at, line + at + length, Self::PRIVATE),
        );
        (!marked).then_some(edit)
    }

    fn word_at(head: &str, word: &str) -> Option<usize> {
        head.match_indices(word)
            .map(|(at, _)| at)
            .find(|&at| Self::is_whole_word(head, at, word.len()))
    }

    fn is_whole_word(head: &str, at: usize, length: usize) -> bool {
        let before = head[..at].chars().next_back();
        let after = head[at + length..].chars().next();
        [before, after]
            .into_iter()
            .flatten()
            .all(|letter| !(letter.is_alphanumeric() | (letter == '_')))
    }
}
