use super::SourceLines;

pub(super) struct OwnMethods;

impl OwnMethods {
    const MODIFIERS: [&'static str; 16] = [
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
        "func",
        "fn",
        "pub",
        "member",
    ];
    const KEYWORDS: [&'static str; 6] = ["def", "function", "fun", "func", "fn", "member"];
    const NOT_BEFORE: [&'static str; 4] = ["=", "(", "return", "new "];
    const MEMBER_MARKS: [char; 2] = ['.', ':'];
    const HEADER_ENDS: [char; 3] = ['{', '(', ','];
    const PARAMETER_STARTS: [char; 2] = ['(', '<'];

    pub(super) fn declares(masked: &str, name: &str) -> bool {
        !Self::starts(masked, name).is_empty()
    }

    pub(super) fn starts(masked: &str, name: &str) -> Vec<usize> {
        SourceLines::with_offsets(masked)
            .into_iter()
            .map(|(at, line)| (at, line.trim_end_matches(['\n', '\r'])))
            .filter(|(_, line)| {
                [Self::declared_on(line, name), Self::typed_on(line, name)].contains(&true)
            })
            .map(|(at, line)| at + SourceLines::indentation(line).len())
            .collect()
    }

    fn typed_on(line: &str, name: &str) -> bool {
        line.match_indices(name)
            .any(|(offset, _)| Self::typed_at(line, offset, name.len()))
    }

    fn typed_at(line: &str, offset: usize, length: usize) -> bool {
        let before = line[..offset].trim();
        let after = line[offset + length..].trim_start();
        let alone = before
            .chars()
            .next_back()
            .is_some_and(|letter| !['.', ':'].contains(&letter));
        let whole = line[offset + length..]
            .chars()
            .next()
            .is_none_or(|letter| !SourceLines::is_word_letter(letter));
        let opens = after.starts_with(Self::PARAMETER_STARTS);
        let plain = !Self::NOT_BEFORE.iter().any(|word| before.contains(word));
        let word_start = line[..offset]
            .chars()
            .next_back()
            .is_none_or(|letter| !SourceLines::is_word_letter(letter));
        [alone, whole, opens, plain, word_start] == [true, true, true, true, true]
    }

    fn declared_on(line: &str, name: &str) -> bool {
        let (keyword, written) = Self::without_modifiers(line);
        let rest = Self::without_receiver(written);
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

    fn without_receiver(rest: &str) -> &str {
        rest.strip_prefix('(')
            .and_then(|inside| inside.split_once(')'))
            .map_or(rest, |(_, after)| after.trim_start())
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
