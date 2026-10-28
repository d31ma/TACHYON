use super::SourceLines;

pub(super) struct ClassClose;

impl ClassClose {
    const CLASS_WORDS: [&'static str; 4] = ["class", "type", "struct", "object"];
    const ATTRIBUTE_OPEN: &'static str = "[[";
    const ATTRIBUTE_CLOSE: &'static str = "]]";
    const MEMBER_STEP: &'static str = "  ";
    const RUBY_END: &'static str = "end";
    const RUBY_STEP: &'static str = "  ";

    pub(super) fn braced(masked: &str, class: &str) -> Option<usize> {
        let declared = Self::declaration(masked, class)?;
        let open = declared + masked[declared..].find('{')?;
        let mut depth = 0_usize;
        masked[open..]
            .char_indices()
            .find(|(_, letter)| Self::closes(*letter, &mut depth))
            .map(|(offset, _)| open + offset)
    }

    pub(super) fn member_slot(masked: &str, class: &str) -> Option<(usize, String)> {
        let at = Self::braced(masked, class)?;
        let line_start = masked[..at].rfind('\n').map_or(0, |offset| offset + 1);
        let indentation = SourceLines::indentation(&masked[line_start..at]);
        Some((at, format!("{indentation}{}", Self::MEMBER_STEP)))
    }

    pub(super) fn indented(masked: &str, class: &str) -> Option<(usize, String)> {
        let (lines, header) = Self::header_line(masked, class)?;
        let level = SourceLines::indentation(lines[header].1).len();
        let body: Vec<&(usize, &str)> = lines[header + 1..]
            .iter()
            .filter(|(_, line)| !SourceLines::is_blank(line))
            .take_while(|(_, line)| SourceLines::indentation(line).len() > level)
            .collect();
        let first = body.first()?;
        let last = body.last()?;
        let end = last.0 + last.1.trim_end_matches(['\n', '\r']).len();
        Some((end, SourceLines::indentation(first.1).to_owned()))
    }

    pub(super) fn ruby_end(masked: &str, class: &str) -> Option<(usize, String)> {
        let (lines, header) = Self::header_line(masked, class)?;
        let indentation = SourceLines::indentation(lines[header].1);
        let end = lines[header + 1..]
            .iter()
            .rev()
            .find(|(_, line)| Self::ends_class(line, indentation))?;
        Some((end.0, format!("{indentation}{}", Self::RUBY_STEP)))
    }

    fn header_line<'a>(masked: &'a str, class: &str) -> Option<(Vec<(usize, &'a str)>, usize)> {
        let declared = Self::declaration(masked, class)?;
        let lines = SourceLines::with_offsets(masked);
        let header = lines
            .iter()
            .position(|(at, line)| at + line.len() > declared)?;
        Some((lines, header))
    }

    fn declaration(masked: &str, class: &str) -> Option<usize> {
        masked
            .match_indices(class)
            .map(|(at, _)| at)
            .find(|at| Self::declares_class(masked, *at, class.len()))
    }

    fn declares_class(masked: &str, at: usize, length: usize) -> bool {
        let before = Self::without_attribute(masked[..at].trim_end());
        let keyword = Self::CLASS_WORDS.iter().any(|word| {
            before.strip_suffix(word).is_some_and(|rest| {
                rest.chars()
                    .next_back()
                    .is_none_or(|letter| !SourceLines::is_word_letter(letter))
            })
        });
        let whole = masked[at + length..]
            .chars()
            .next()
            .is_none_or(|letter| !SourceLines::is_word_letter(letter));
        [keyword, whole] == [true, true]
    }

    fn without_attribute(before: &str) -> &str {
        before
            .strip_suffix(Self::ATTRIBUTE_CLOSE)
            .and_then(|inside| inside.rfind(Self::ATTRIBUTE_OPEN))
            .map_or(before, |open| before[..open].trim_end())
    }

    fn closes(letter: char, depth: &mut usize) -> bool {
        let opened = usize::from(letter == '{');
        let closed = usize::from(letter == '}');
        *depth = (*depth + opened).saturating_sub(closed);
        [closed == 1, *depth == 0] == [true, true]
    }

    fn ends_class(line: &str, indentation: &str) -> bool {
        let same_level = SourceLines::indentation(line) == indentation;
        [line.trim() == Self::RUBY_END, same_level] == [true, true]
    }
}
