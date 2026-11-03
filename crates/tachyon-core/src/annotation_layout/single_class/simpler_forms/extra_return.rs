#![allow(clippy::needless_bitwise_bool)]

use super::block_text::BlockText;
use super::code_language::CodeLanguage;
use super::form_pattern::FormPattern;
use super::if_part::IfPart;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct ExtraReturn;

impl ExtraReturn {
    const SKIPPED: [CodeLanguage; 2] = [CodeLanguage::Go, CodeLanguage::Ruby];
    const EMPTY_VALUES: [&'static str; 2] = [";", "None"];
    const SMALLEST_BODY: usize = 2;
    const CONTROL: [&'static str; 24] = [
        "if", "else", "for", "foreach", "while", "switch", "catch", "try", "do", "finally", "with",
        "using", "lock", "}", "case", "default", "match", "loop", "unsafe", "defer", "select",
        "guard", "repeat", "when",
    ];
    const FUNCTION_WORDS: [(CodeLanguage, &'static str); 4] = [
        (CodeLanguage::Kotlin, "fun"),
        (CodeLanguage::Swift, "func"),
        (CodeLanguage::Rust, "fn"),
        (CodeLanguage::Php, "function"),
    ];

    pub(super) fn find(pieces: &Pieces<'_>) -> Vec<SimplerForm> {
        let wanted = !pieces.language().is_one_of(&Self::SKIPPED);
        (0..pieces.len())
            .filter(|_| wanted)
            .filter_map(|at| Self::body(pieces, at))
            .filter_map(|body| Self::form(pieces, &body))
            .collect()
    }

    fn body(pieces: &Pieces<'_>, at: usize) -> Option<IfPart> {
        if pieces.language().is_python() {
            return Self::python_body(pieces, at);
        }
        Self::braced_body(pieces, at)
    }

    fn python_body(pieces: &Pieces<'_>, at: usize) -> Option<IfPart> {
        let keyword =
            Some(at).filter(|&index| pieces.is(index, "def") & pieces.line_start(index))?;
        let colon = pieces.line_after(keyword) - 1;
        let column = pieces.column(keyword);
        Some(colon)
            .filter(|&index| pieces.is(index, ":"))
            .and_then(|index| IfPart::indented(pieces, index, column))
    }

    fn braced_body(pieces: &Pieces<'_>, at: usize) -> Option<IfPart> {
        let open = Some(at).filter(|&index| pieces.is(index, "{"))?;
        let close = pieces.partner(open)?;
        let first = pieces.line_first(open);
        let same_depth = pieces.depth(open) == pieces.depth(first);
        let ends_line = pieces.line_start(open + 1);
        let control = pieces.is_any(first, &Self::CONTROL);
        let named = pieces
            .top(first, open)
            .into_iter()
            .any(|index| Self::is_named_call(pieces, index));
        let keyword = Self::FUNCTION_WORDS
            .iter()
            .find(|(language, _)| *language == pieces.language())
            .is_none_or(|(_, word)| pieces.has_top(first, open, &[word]));
        let function = same_depth & ends_line & !control & named & keyword;
        function.then_some(IfPart {
            open: Some(open),
            from: open + 1,
            to: close,
            close: Some(close),
            end: close + 1,
        })
    }

    fn is_named_call(pieces: &Pieces<'_>, at: usize) -> bool {
        let before = at.wrapping_sub(1);
        pieces.is(at, "(") & pieces.is_name(before) & !pieces.is_any(before, &Self::CONTROL)
    }

    fn returns_value(pieces: &Pieces<'_>, at: usize) -> bool {
        let next = at + 1;
        let empty = pieces.is_any(next, &Self::EMPTY_VALUES) | pieces.line_start(next);
        pieces.is(at, "return") & !empty
    }

    fn form(pieces: &Pieces<'_>, body: &IfPart) -> Option<SimplerForm> {
        let (first, end) = (body.from, body.to);
        let last = BlockText::last_statement(pieces, first, end);
        let statement_end = pieces.line_after(last);
        let value = (last + 1..statement_end).all(|at| pieces.is_any(at, &Self::EMPTY_VALUES));
        let bare = pieces.is(last, "return")
            & (statement_end == end)
            & (statement_end <= last + 2)
            & value;
        let kept = BlockText::statement_count(pieces, first, end) >= Self::SMALLEST_BODY;
        let consistent = !(first..last).any(|at| Self::returns_value(pieces, at));
        let fits = bare & kept & consistent & pieces.is_whole_lines(last, statement_end);
        fits.then(|| {
            SimplerForm::new(
                FormPattern::ExtraReturn,
                pieces.line_head(last),
                pieces.line_tail(statement_end - 1),
                String::new(),
            )
        })
    }
}
