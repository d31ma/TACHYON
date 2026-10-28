use super::{MarkerStrip, ParameterSplit, SourceLines};

pub(super) struct MethodHeader {
    pub(super) declaration: String,
    pub(super) parameters: Vec<String>,
    pub(super) returns_value: bool,
}

impl MethodHeader {
    const EXPRESSION_OPENERS: [(&'static str, &'static str); 4] =
        [("cs", "=>"), ("dart", "=>"), ("kt", "="), ("fs", "=")];
    const DROPPED: [(&'static str, &'static str); 11] = [
        ("cs", "override"),
        ("cs", "virtual"),
        ("cs", "abstract"),
        ("cs", "sealed"),
        ("cs", "async"),
        ("kt", "override"),
        ("kt", "open"),
        ("kt", "abstract"),
        ("dart", "async"),
        ("hpp", "virtual"),
        ("hpp", "override"),
    ];
    const DART_BODY_MARKS: [&'static str; 3] = ["async*", "sync*", "async"];
    const VOID: &'static str = "void";
    const SCAN_LIMIT: usize = 4096;

    pub(super) fn read(
        language: &str,
        (source, masked): (&str, &str),
        start: usize,
        name: &str,
    ) -> Option<Self> {
        let name_at = Self::name_at(masked, start, name)?;
        let open = name_at + masked[name_at..].find('(')?;
        let close = ParameterSplit::matching_close(masked, open)?;
        let end = Self::body_start(language, masked, close)?;
        let parameters = ParameterSplit::parts(source, masked, open + 1..close);
        let declaration = Self::declaration(language, &source[start..end]);
        let returns_value =
            Self::returns_value(language, &masked[start..name_at], &masked[close + 1..end]);
        Some(Self {
            declaration,
            parameters,
            returns_value,
        })
    }

    fn name_at(masked: &str, start: usize, name: &str) -> Option<usize> {
        let (_, line_end) = SourceLines::line_around(masked, start);
        let line = &masked[start..line_end];
        line.match_indices(name)
            .map(|(offset, _)| offset)
            .find(|offset| Self::names_method(line, *offset, name.len()))
            .map(|offset| start + offset)
    }

    fn names_method(line: &str, offset: usize, length: usize) -> bool {
        let before = line[..offset].chars().next_back();
        let after = line[offset + length..].trim_start();
        let alone = before.is_none_or(|letter| !SourceLines::is_word_letter(letter));
        let called = after.starts_with(['(', '<']);
        [alone, called] == [true, true]
    }

    fn body_start(language: &str, masked: &str, close: usize) -> Option<usize> {
        let opener = Self::EXPRESSION_OPENERS
            .iter()
            .find(|(owner, _)| *owner == language)
            .map(|(_, opener)| *opener);
        let rest = &masked[close + 1..masked.len().min(close + 1 + Self::SCAN_LIMIT)];
        let mut depth = 0_usize;
        rest.char_indices()
            .find(|(offset, letter)| {
                depth = Self::depth_after(depth, *letter);
                let at_level = depth == 0;
                let block = *letter == '{';
                let expression =
                    opener.is_some_and(|opener| Self::opens_expression(rest, *offset, opener));
                let ended = *letter == ';';
                [at_level, [block, expression, ended].contains(&true)] == [true, true]
            })
            .filter(|(_, letter)| *letter != ';')
            .map(|(offset, _)| close + 1 + offset)
    }

    fn depth_after(depth: usize, letter: char) -> usize {
        let opened = usize::from(['(', '['].contains(&letter));
        let closed = usize::from([')', ']'].contains(&letter));
        (depth + opened).saturating_sub(closed)
    }

    fn opens_expression(rest: &str, offset: usize, opener: &str) -> bool {
        let starts = rest[offset..].starts_with(opener);
        let before = rest[..offset].chars().next_back();
        let after = rest[offset + opener.len()..].chars().next();
        let lone_before = before.is_none_or(|letter| !"=!<>".contains(letter));
        let lone_after = after.is_none_or(|letter| !"=>".contains(letter));
        [starts, lone_before, lone_after] == [true, true, true]
    }

    fn declaration(language: &str, written: &str) -> String {
        let plain = MarkerStrip::plain(language, written);
        let kept: Vec<&str> = plain
            .split_whitespace()
            .filter(|word| !Self::DROPPED.contains(&(language, *word)))
            .filter(|word| !Self::is_dart_body_mark(language, word))
            .collect();
        kept.join(" ").replace("( ", "(")
    }

    fn is_dart_body_mark(language: &str, word: &str) -> bool {
        [language == "dart", Self::DART_BODY_MARKS.contains(&word)] == [true, true]
    }

    fn returns_value(language: &str, before_name: &str, after_parameters: &str) -> bool {
        match language {
            "go" => !after_parameters.trim().is_empty(),
            _ => before_name.split_whitespace().next_back() != Some(Self::VOID),
        }
    }
}
