use super::YonConvention;
use std::path::Path;

pub(super) struct HandlerBody;

impl HandlerBody {
    const DECLARATION_KEYWORDS: [&str; 4] = ["class", "object", "impl", "struct"];
    const HANDLER: &str = "Handler";

    pub(super) fn find(source: &Path, contents: &str) -> String {
        let python = YonConvention::extension(source) == "py";
        Self::body_for_language(python, contents)
    }

    fn body_for_language(python: bool, contents: &str) -> String {
        if python {
            Self::python_body(contents)
        } else {
            Self::brace_bodies(contents)
        }
    }

    fn brace_bodies(contents: &str) -> String {
        let mut bodies = String::new();
        let mut cursor = 0_usize;
        while let Some(relative) = contents[cursor..].find(Self::HANDLER) {
            cursor = Self::read_handler(contents, cursor + relative, &mut bodies);
        }
        bodies
    }

    fn read_handler(contents: &str, handler_start: usize, bodies: &mut String) -> usize {
        let word_end = handler_start + Self::HANDLER.len();
        let block = Self::handler_block(contents, handler_start, word_end);
        Self::take_block(contents, block, word_end, bodies)
    }

    fn handler_block(
        contents: &str,
        handler_start: usize,
        word_end: usize,
    ) -> Option<(usize, usize)> {
        let declared = Self::is_declared_handler(contents, handler_start, word_end);
        declared
            .then(|| Self::block_after(contents, word_end))
            .flatten()
    }

    fn is_declared_handler(contents: &str, handler_start: usize, word_end: usize) -> bool {
        YonConvention::is_whole_word(contents, handler_start, word_end)
            && Self::is_handler_declaration(contents, handler_start)
    }

    fn take_block(
        contents: &str,
        block: Option<(usize, usize)>,
        word_end: usize,
        bodies: &mut String,
    ) -> usize {
        match block {
            Some((open, close)) => {
                bodies.push_str(&contents[open + 1..close]);
                bodies.push('\n');
                close + 1
            }
            None => word_end,
        }
    }

    fn is_handler_declaration(contents: &str, handler_start: usize) -> bool {
        let line_start = contents[..handler_start]
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        let prefix = contents[line_start..handler_start].trim();
        prefix
            .split_whitespace()
            .any(|word| Self::DECLARATION_KEYWORDS.contains(&word))
    }

    fn block_after(contents: &str, from: usize) -> Option<(usize, usize)> {
        let open = from + contents[from..].find(['{', ';'])?;
        contents
            .as_bytes()
            .get(open)
            .filter(|byte| **byte == b'{')?;
        let pairs = contents.bytes().map(char::from).enumerate().skip(open);
        let close = YonConvention::closing_index(pairs, '{', '}')?;
        Some((open, close))
    }

    fn python_body(contents: &str) -> String {
        let lines = contents.lines().collect::<Vec<_>>();
        lines
            .iter()
            .position(|line| line.trim_start().starts_with("class Handler"))
            .map_or_else(String::new, |index| {
                Self::python_class_body(&lines[index + 1..], Self::indent_width(lines[index]))
            })
    }

    fn python_class_body(lines: &[&str], class_indent: usize) -> String {
        lines
            .iter()
            .take_while(|line| Self::inside_class(line, class_indent))
            .map(|line| Self::body_line(line))
            .collect()
    }

    fn inside_class(line: &str, class_indent: usize) -> bool {
        line.trim().is_empty() || Self::indent_width(line) > class_indent
    }

    fn body_line(line: &str) -> String {
        if line.trim().is_empty() {
            return String::from("\n");
        }
        format!("{line}\n")
    }

    fn indent_width(line: &str) -> usize {
        line.len() - line.trim_start().len()
    }
}
