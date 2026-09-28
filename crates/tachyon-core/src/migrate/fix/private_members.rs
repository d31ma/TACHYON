#![allow(clippy::needless_bitwise_bool)]

use super::edits::TextEdit;
use super::private_keyword::PrivateKeyword;
use super::private_rename::PrivateRename;
use super::scan::SourceText;
use crate::annotation_layout::UnusedMember;
use std::path::Path;

pub(super) struct PrivateMembers;

impl PrivateMembers {
    const RENAMED: [&'static str; 4] = ["js", "py", "dart", "go"];

    pub(super) fn plan(
        relative: &str,
        source: &str,
        members: &[UnusedMember],
    ) -> Vec<(Vec<TextEdit>, String)> {
        let path = Path::new(relative);
        let language = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let class = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let text = SourceText::read(language, source);
        Self::planned(text.as_ref(), (language, class), relative, members)
    }

    fn planned(
        text: Option<&SourceText<'_>>,
        owner: (&str, &str),
        relative: &str,
        members: &[UnusedMember],
    ) -> Vec<(Vec<TextEdit>, String)> {
        let Some(text) = text else {
            return Vec::new();
        };
        members
            .iter()
            .filter(|member| member.path == relative)
            .filter_map(|member| Self::member_plan(text, owner, member))
            .collect()
    }

    fn member_plan(
        text: &SourceText<'_>,
        owner: (&str, &str),
        member: &UnusedMember,
    ) -> Option<(Vec<TextEdit>, String)> {
        let (_, class) = owner;
        let edits = Self::edits(text, owner, member)?;
        let note = format!(
            "Made `{}` on line {} private: nothing outside {class} uses it (TY1333).",
            member.name,
            text.line_number(member.start)
        );
        Some((edits, note))
    }

    fn edits(
        text: &SourceText<'_>,
        owner: (&str, &str),
        member: &UnusedMember,
    ) -> Option<Vec<TextEdit>> {
        let (language, _) = owner;
        if Self::RENAMED.contains(&language) {
            return PrivateRename::edits(text, owner, member);
        }
        PrivateKeyword::edits(text, language, member)
    }
}
