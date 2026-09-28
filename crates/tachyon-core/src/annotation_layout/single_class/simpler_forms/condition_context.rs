#![allow(clippy::needless_bitwise_bool)]

use super::if_statement::IfStatement;
use super::pieces::Pieces;

pub(super) struct ConditionContext {
    ranges: Vec<(usize, usize)>,
}

impl ConditionContext {
    const JOINERS: [&'static str; 4] = ["&&", "||", "and", "or"];

    pub(super) fn new(statements: &[IfStatement]) -> Self {
        Self {
            ranges: statements
                .iter()
                .map(|statement| (statement.condition_from, statement.condition_to))
                .collect(),
        }
    }

    pub(super) fn starts_operand(&self, pieces: &Pieces<'_>, at: usize) -> bool {
        let after = self.ranges.partition_point(|&(from, _)| from <= at);
        self.ranges
            .get(after.wrapping_sub(1))
            .is_some_and(|&(from, to)| Self::is_operand_start(pieces, from, to, at))
    }

    pub(super) fn operand_end(pieces: &Pieces<'_>, at: usize) -> usize {
        let depth = pieces.depth(at);
        (at..pieces.len())
            .find(|&index| Self::ends_operand(pieces, index, depth))
            .unwrap_or(pieces.len())
    }

    fn is_operand_start(pieces: &Pieces<'_>, from: usize, to: usize, at: usize) -> bool {
        let inside = (from <= at) & (at < to) & (pieces.depth(at) == pieces.depth(from));
        let first = (at == from) | pieces.is_any(at.wrapping_sub(1), &Self::JOINERS);
        inside & first
    }

    fn ends_operand(pieces: &Pieces<'_>, at: usize, depth: usize) -> bool {
        let joiner = (pieces.depth(at) == depth) & pieces.is_any(at, &Self::JOINERS);
        let colon = (pieces.depth(at) == depth) & pieces.is(at, ":") & pieces.line_start(at + 1);
        let brace = (pieces.depth(at) == depth) & pieces.is(at, "{");
        (pieces.depth(at) < depth) | joiner | colon | brace
    }
}
