use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;

pub(super) struct ModuleCode {
    pub(super) text: String,
    pub(super) code: String,
}

impl ModuleCode {
    const PATH: &'static str = "service.js";
    const IMPORT: &'static str = "import";
    const STATEMENT_END: char = ';';
    const KEYWORDS: [&'static str; 46] = [
        "as",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "debugger",
        "default",
        "delete",
        "do",
        "else",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "from",
        "function",
        "get",
        "if",
        "import",
        "in",
        "instanceof",
        "let",
        "new",
        "null",
        "of",
        "return",
        "set",
        "static",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "typeof",
        "undefined",
        "var",
        "void",
        "while",
        "with",
        "yield",
    ];

    pub(super) fn read(bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let masked = crate::annotation_layout::masked_source(Path::new(Self::PATH), &text);
        let code = masked.unwrap_or_else(|_| " ".repeat(text.len()));
        Self { text, code }
    }

    pub(super) fn words(code: &str) -> BTreeSet<String> {
        code.split(|character| !Self::is_word_character(character))
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect()
    }

    pub(super) fn top_level_names(&self, without: Range<usize>) -> BTreeSet<String> {
        let mut code = self.code.clone();
        code.replace_range(without.clone(), &" ".repeat(without.len()));
        let mut names = Self::words(&Self::outside_brackets(&code));
        names.extend(Self::import_words(&code));
        names.retain(|name| !Self::KEYWORDS.contains(&name.as_str()));
        names
    }

    pub(super) fn matching_brace(&self, open: usize) -> Option<usize> {
        let mut depth = 0_usize;
        let offset = self.code.as_bytes()[open..]
            .iter()
            .map(|byte| {
                depth = Self::brace_depth(depth, *byte);
                depth
            })
            .position(|depth| depth == 0)?;
        Some(open + offset)
    }

    fn is_word_character(character: char) -> bool {
        [character.is_alphanumeric(), "_$".contains(character)].contains(&true)
    }

    fn outside_brackets(code: &str) -> String {
        let mut depth = 0_usize;
        let mut kept = String::with_capacity(code.len());
        for character in code.chars() {
            depth = Self::bracket_depth(depth, character);
            kept.push(Self::kept_at_depth(character, depth));
        }
        kept
    }

    fn bracket_depth(depth: usize, character: char) -> usize {
        match character {
            '{' | '(' | '[' => depth + 1,
            '}' | ')' | ']' => depth.saturating_sub(1),
            _ => depth,
        }
    }

    fn kept_at_depth(character: char, depth: usize) -> char {
        if depth == 0 { character } else { ' ' }
    }

    fn brace_depth(depth: usize, byte: u8) -> usize {
        match byte {
            b'{' => depth + 1,
            b'}' => depth.saturating_sub(1),
            _ => depth,
        }
    }

    fn import_words(code: &str) -> BTreeSet<String> {
        code.match_indices(Self::IMPORT)
            .flat_map(|(at, _)| Self::words(Self::statement_at(code, at)))
            .collect()
    }

    fn statement_at(code: &str, at: usize) -> &str {
        let rest = &code[at..];
        let end = rest.find(Self::STATEMENT_END).unwrap_or(rest.len());
        &rest[..end]
    }
}
