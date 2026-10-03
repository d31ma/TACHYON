use crate::annotation_layout::single_class::Tokens;

pub(super) struct DomStatement;

impl DomStatement {
    const END: [&'static str; 3] = [";", "}", ","];
    const START: [&'static str; 4] = [";", "{", "}", ","];
    const CONTINUATION: &'static [&'static str] = &[
        ".", "?", ":", "+", "-", "*", "/", "%", "&", "|", "^", "=", "<", ">",
    ];
    const PREFIX: &'static [&'static str] =
        &["(", "[", "{", "!", "~", "new", "await", "yield", "return"];

    pub(super) fn value_end(tokens: &Tokens<'_>, start: usize) -> usize {
        Self::find_end(tokens, start)
    }

    pub(super) fn assignment_start(tokens: &Tokens<'_>, at: usize) -> usize {
        Self::find_start(tokens, at)
    }

    fn find_start(tokens: &Tokens<'_>, at: usize) -> usize {
        (0..at)
            .rev()
            .find(|before| {
                [
                    Self::START.contains(&tokens.items[*before].text),
                    Self::ends_at_newline(tokens, 0, *before),
                ]
                .contains(&true)
            })
            .map_or(0, |before| {
                Self::END
                    .contains(&tokens.items[before].text)
                    .then_some(before + 1)
                    .unwrap_or(before)
            })
    }

    fn find_end(tokens: &Tokens<'_>, start: usize) -> usize {
        let mut depth = 0_usize;
        (start..tokens.items.len())
            .find(|at| {
                let boundary = Self::ends_value(tokens, start, *at, depth);
                depth = Self::depth_after(depth, tokens.items[*at].text);
                boundary
            })
            .unwrap_or(tokens.items.len())
    }

    fn ends_value(tokens: &Tokens<'_>, start: usize, at: usize, depth: usize) -> bool {
        let end = Self::END.contains(&tokens.items[at].text);
        let newline = Self::ends_at_newline(tokens, start, at);
        [depth == 0, [end, newline].contains(&true)]
            .into_iter()
            .all(|holds| holds)
    }

    fn ends_at_newline(tokens: &Tokens<'_>, start: usize, at: usize) -> bool {
        let previous = at
            .checked_sub(1)
            .and_then(|before| tokens.items.get(before))
            .map_or("", |token| token.text);
        let next = tokens.items[at].text;
        let continues_before = [
            Self::CONTINUATION.contains(&previous),
            Self::PREFIX.contains(&previous),
        ]
        .contains(&true);
        let continues_after = [
            Self::CONTINUATION.contains(&next),
            ["(", "["].contains(&next),
        ]
        .contains(&true);
        [
            at > start,
            tokens.new_line_before(at),
            !continues_before,
            !continues_after,
        ]
        .into_iter()
        .all(|holds| holds)
    }

    fn depth_after(depth: usize, text: &str) -> usize {
        let opens = usize::from(["(", "[", "{"].contains(&text));
        let closes = usize::from([")", "]", "}"].contains(&text));
        (depth + opens).saturating_sub(closes)
    }
}
