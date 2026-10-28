use super::SourceLines;

pub(super) struct OwnMethods;

impl OwnMethods {
    const MODIFIERS: [&'static str; 12] = [
        "public",
        "private",
        "protected",
        "static",
        "async",
        "override",
        "readonly",
        "final",
        "abstract",
        "def",
        "function",
        "fun",
    ];
    const KEYWORDS: [&'static str; 3] = ["def", "function", "fun"];
    const MEMBER_MARKS: [char; 2] = ['.', ':'];
    const HEADER_ENDS: [char; 3] = ['{', '(', ','];
    const PARAMETER_STARTS: [char; 2] = ['(', '<'];

    pub(super) fn declares(masked: &str, name: &str) -> bool {
        masked.lines().any(|line| Self::declared_on(line, name))
    }

    fn declared_on(line: &str, name: &str) -> bool {
        let (keyword, rest) = Self::without_modifiers(line);
        let head_end = rest
            .find(|letter: char| {
                ![
                    SourceLines::is_word_letter(letter),
                    Self::MEMBER_MARKS.contains(&letter),
                    letter == '$',
                ]
                .contains(&true)
            })
            .unwrap_or(rest.len());
        let head = &rest[..head_end];
        let member = head.rsplit(Self::MEMBER_MARKS).next().unwrap_or(head);
        let plain = !head.contains(Self::MEMBER_MARKS);
        let after = rest[head_end..].trim_start();
        let opens = after.starts_with(Self::PARAMETER_STARTS);
        let header = line.trim_end().ends_with(Self::HEADER_ENDS);
        let braced = [plain, opens, header] == [true, true, true];
        [member == name, [keyword, braced].contains(&true)] == [true, true]
    }

    fn without_modifiers(line: &str) -> (bool, &str) {
        let mut rest = line.trim_start();
        let mut keyword = false;
        while let Some((word, after)) = rest
            .split_once(' ')
            .filter(|(word, _)| Self::MODIFIERS.contains(word))
        {
            keyword |= Self::KEYWORDS.contains(&word);
            rest = after.trim_start();
        }
        (keyword, rest)
    }
}
