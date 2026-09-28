#![allow(clippy::needless_bitwise_bool)]

use super::pieces::Pieces;

#[derive(Clone, Copy, Debug)]
pub(super) struct IfPart {
    pub(super) open: Option<usize>,
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) close: Option<usize>,
    pub(super) end: usize,
}

impl IfPart {
    const ENDED_STOPS: [&'static str; 3] = ["else", "elsif", "end"];

    pub(super) fn after_head(pieces: &Pieces<'_>, at: usize) -> Option<Self> {
        Self::braced_or_loose(pieces, at)
    }

    pub(super) fn indented(pieces: &Pieces<'_>, colon: usize, column: usize) -> Option<Self> {
        let from = colon + 1;
        let starts_below = pieces.line_start(from) & (pieces.column(from) > column);
        let end = (from + 1..pieces.len())
            .find(|&at| pieces.line_start(at) & (pieces.column(at) <= column))
            .unwrap_or(pieces.len());
        starts_below.then_some(Self {
            open: Some(colon),
            from,
            to: end,
            close: None,
            end,
        })
    }

    pub(super) fn ended(pieces: &Pieces<'_>, from: usize, column: usize) -> Option<Self> {
        let starts_below = pieces.line_start(from) & (pieces.column(from) > column);
        let stop = (from + 1..pieces.len())
            .find(|&at| pieces.line_start(at) & (pieces.column(at) <= column))?;
        let closes = pieces.is_any(stop, &Self::ENDED_STOPS) & (pieces.column(stop) == column);
        (starts_below & closes).then_some(Self {
            open: None,
            from,
            to: stop,
            close: Some(stop),
            end: stop,
        })
    }

    pub(super) fn is_braced(&self) -> bool {
        self.open.is_some() & self.close.is_some()
    }

    fn braced_or_loose(pieces: &Pieces<'_>, at: usize) -> Option<Self> {
        if pieces.is(at, "{") {
            return Self::block(pieces, at);
        }
        Self::loose(pieces, at)
    }

    fn block(pieces: &Pieces<'_>, open: usize) -> Option<Self> {
        let close = pieces.partner(open)?;
        Some(Self {
            open: Some(open),
            from: open + 1,
            to: close,
            close: Some(close),
            end: close + 1,
        })
    }

    fn loose(pieces: &Pieces<'_>, at: usize) -> Option<Self> {
        let line_end = pieces.line_after(at);
        let stop = pieces
            .find_top(at, line_end, &["else", ";"])
            .unwrap_or(line_end);
        let end = stop + usize::from(pieces.is(stop, ";"));
        let balanced = pieces.depth(end) <= pieces.depth(at);
        ((end > at) & balanced).then_some(Self {
            open: None,
            from: at,
            to: end,
            close: None,
            end,
        })
    }
}
