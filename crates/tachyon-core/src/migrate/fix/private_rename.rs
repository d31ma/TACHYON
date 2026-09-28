#![allow(clippy::needless_bitwise_bool)]

use super::edits::TextEdit;
use super::member_reference::MemberReference;
use super::scan::SourceText;
use crate::annotation_layout::UnusedMember;

pub(super) struct PrivateRename;

impl PrivateRename {
    const OWN_RECEIVERS: [&'static str; 6] = ["this", "self", "cls", "$this", "Self", "static"];
    const ACCESS_MARKS: [&'static str; 4] = ["?.", "!.", "::", "."];
    const SPREAD: &'static str = "..";
    const GO_RECEIVER: &'static str = "func (";
    const GO: &'static str = "go";
    const DART: &'static str = "dart";
    const REFERENCE_WORDS: [&'static str; 9] = [
        "return", "await", "yield", "else", "throw", "case", "in", "is", "as",
    ];
    const UNDERSCORE: &'static str = "_";

    pub(super) fn edits(
        text: &SourceText<'_>,
        owner: (&str, &str),
        member: &UnusedMember,
    ) -> Option<Vec<TextEdit>> {
        let (language, _) = owner;
        let private = Self::private_name(language, &member.name);
        let references = Self::references(text, owner, member);
        let unsure = Self::has_unsure(&references);
        let taken = Self::is_taken(text, &private);
        let edits = Self::rename_edits(language, member, &private, &references);
        (!(unsure | taken)).then_some(edits)
    }

    fn references(
        text: &SourceText<'_>,
        owner: (&str, &str),
        member: &UnusedMember,
    ) -> Vec<(usize, MemberReference)> {
        let (language, _) = owner;
        let receivers = Self::receivers(text, owner);
        text.code_words(&member.name)
            .into_iter()
            .filter(|&at| at != member.start)
            .map(|at| (at, Self::reference(text, (language, &receivers), at)))
            .collect()
    }

    fn has_unsure(references: &[(usize, MemberReference)]) -> bool {
        references
            .iter()
            .any(|(_, kind)| *kind == MemberReference::Unsure)
    }

    fn is_taken(text: &SourceText<'_>, private: &str) -> bool {
        !text.code_words(private).is_empty()
    }

    fn rename_edits(
        language: &str,
        member: &UnusedMember,
        private: &str,
        references: &[(usize, MemberReference)],
    ) -> Vec<TextEdit> {
        let renamed = references
            .iter()
            .filter(|(_, kind)| *kind == MemberReference::Rename)
            .map(|(at, _)| *at);
        std::iter::once(member.start)
            .chain(renamed)
            .map(|at| Self::renamed_at(at, language, &member.name, private))
            .collect()
    }

    fn private_name(language: &str, name: &str) -> String {
        if language == Self::GO {
            return Self::lowercase_first(name);
        }
        format!("{}{name}", Self::UNDERSCORE)
    }

    fn lowercase_first(name: &str) -> String {
        let mut letters = name.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_lowercase().chain(letters).collect()
        })
    }

    fn renamed_at(at: usize, language: &str, name: &str, private: &str) -> TextEdit {
        if language == Self::GO {
            return TextEdit::replace(at, at + name.len(), private);
        }
        let prefix = &private[..private.len() - name.len()];
        TextEdit::insert(at, prefix)
    }

    fn receivers<'a>(text: &SourceText<'a>, owner: (&str, &'a str)) -> Vec<&'a str> {
        let (_, class) = owner;
        let mut receivers = Self::OWN_RECEIVERS.to_vec();
        receivers.push(class);
        receivers.extend(
            text.source
                .match_indices(Self::GO_RECEIVER)
                .map(|(at, _)| Self::word_after(text.source, at + Self::GO_RECEIVER.len())),
        );
        receivers
    }

    fn word_after(source: &str, start: usize) -> &str {
        let rest = &source[start..];
        let end = rest
            .find(|letter: char| !(letter.is_alphanumeric() | (letter == '_')))
            .unwrap_or(rest.len());
        &rest[..end]
    }

    fn reference(text: &SourceText<'_>, context: (&str, &[&str]), at: usize) -> MemberReference {
        let (language, receivers) = context;
        let before = Self::code_before(text, at);
        let spread = before.ends_with(Self::SPREAD);
        let mark = Self::ACCESS_MARKS
            .iter()
            .find(|mark| before.ends_with(*mark))
            .filter(|_| !spread);
        mark.map_or_else(
            || Self::bare_reference(text, language, at),
            |mark| Self::accessed_reference(text, receivers, before.len() - mark.len()),
        )
    }

    fn accessed_reference(
        text: &SourceText<'_>,
        receivers: &[&str],
        mark_at: usize,
    ) -> MemberReference {
        let receiver = text.previous_word(mark_at);
        let own = receivers.contains(&receiver) & !receiver.is_empty();
        [MemberReference::Keep, MemberReference::Rename][usize::from(own)]
    }

    fn bare_reference(text: &SourceText<'_>, language: &str, at: usize) -> MemberReference {
        let before = text.previous_word(at);
        let declared = !(before.is_empty() | Self::REFERENCE_WORDS.contains(&before));
        match language {
            Self::DART => [MemberReference::Rename, MemberReference::Unsure][usize::from(declared)],
            Self::GO => MemberReference::Unsure,
            _ => MemberReference::Keep,
        }
    }

    fn code_before<'a>(text: &SourceText<'a>, at: usize) -> &'a str {
        let end = text.previous_code(at).map_or(0, |last| last + 1);
        &text.source[..end]
    }
}
