use super::super::class_file::ClassFile;

pub(super) struct TypeComments {
    pub(super) comments: String,
    pub(super) markers: String,
    pub(super) top: usize,
    pub(super) line: usize,
    pub(super) indent: String,
}

impl TypeComments {
    const NEWLINE: char = '\n';
    const HASH_LANGUAGES: [&'static str; 2] = ["py", "rb"];
    const SLASH_STARTS: [&'static str; 3] = ["/*", "*", "//"];
    const HASH_STARTS: [&'static str; 1] = ["#"];
    const ANNOTATION_STARTS: [&'static str; 2] = ["@", "#["];
    const INDENT: [char; 2] = [' ', '\t'];

    pub(super) fn above(class: &ClassFile, at: usize) -> Self {
        let line = Self::line_start(class.text, class.words.start(at));
        let lines = Self::lines_above(class, line);
        let top = lines.last().map_or(line, |(offset, _)| *offset);
        Self {
            comments: Self::joined(&lines, false),
            markers: Self::joined(&lines, true),
            top,
            line,
            indent: Self::indent(&class.text[line..]),
        }
    }

    fn line_start(text: &str, offset: usize) -> usize {
        text[..offset]
            .rfind(Self::NEWLINE)
            .map_or(0, |newline| newline + 1)
    }

    fn lines_above<'t>(class: &ClassFile<'t>, line: usize) -> Vec<(usize, &'t str)> {
        let hash = Self::HASH_LANGUAGES.contains(&class.extension);
        Self::numbered(&class.text[..line])
            .into_iter()
            .rev()
            .take_while(|(_, text)| Self::belongs(text.trim(), hash))
            .collect()
    }

    fn numbered(text: &str) -> Vec<(usize, &str)> {
        let mut offset = 0;
        let mut lines = Vec::new();
        for line in text.split_inclusive(Self::NEWLINE) {
            lines.push((offset, line));
            offset += line.len();
        }
        lines
    }

    fn belongs(text: &str, hash: bool) -> bool {
        Self::is_annotation(text) | Self::is_comment(text, hash)
    }

    fn is_comment(text: &str, hash: bool) -> bool {
        let starts: [&[&str]; 2] = [&Self::SLASH_STARTS, &Self::HASH_STARTS];
        starts[usize::from(hash)]
            .iter()
            .any(|start| text.starts_with(start))
    }

    fn is_annotation(text: &str) -> bool {
        Self::ANNOTATION_STARTS
            .iter()
            .any(|start| text.starts_with(start))
    }

    fn joined(lines: &[(usize, &str)], annotations: bool) -> String {
        lines
            .iter()
            .rev()
            .filter(|(_, text)| Self::is_annotation(text.trim()) == annotations)
            .map(|(_, text)| *text)
            .collect()
    }

    fn indent(line: &str) -> String {
        let width = line.len() - line.trim_start_matches(Self::INDENT).len();
        line[..width].to_owned()
    }
}
