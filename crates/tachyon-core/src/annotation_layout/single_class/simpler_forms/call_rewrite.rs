#![allow(clippy::needless_bitwise_bool)]

use super::call_match::CallMatch;
use super::call_rule::CallRule;
use super::expression_edges::ExpressionEdges;
use super::form_pattern::FormPattern;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;

pub(super) struct CallRewrite;

impl CallRewrite {
    pub(super) fn forms(
        pieces: &Pieces<'_>,
        rules: &[CallRule],
        pattern: FormPattern,
    ) -> Vec<SimplerForm> {
        Self::matches(pieces, rules)
            .iter()
            .map(|found| Self::form(pieces, found, pattern))
            .collect()
    }

    pub(super) fn matches<'r>(pieces: &Pieces<'_>, rules: &'r [CallRule]) -> Vec<CallMatch<'r>> {
        let language = pieces.language();
        rules
            .iter()
            .filter(|rule| rule.languages.contains(&language))
            .flat_map(|rule| {
                (0..pieces.len()).filter_map(move |at| Self::matched(pieces, rule, at))
            })
            .collect()
    }

    pub(super) fn receiver_text<'a>(pieces: &Pieces<'a>, found: &CallMatch<'_>) -> &'a str {
        pieces.slice(found.start, found.name - 1)
    }

    fn form(pieces: &Pieces<'_>, found: &CallMatch<'_>, pattern: FormPattern) -> SimplerForm {
        let rule = found.rule;
        let not = ["", pieces.language().not_word()][usize::from(rule.negated)];
        let prefix = pieces.bytes(pieces.start(found.start), pieces.start(found.name));
        let group = pieces
            .bytes(pieces.end(found.name), pieces.end(found.group_end - 1))
            .replacen(rule.dropped, "", 1);
        SimplerForm::new(
            pattern,
            pieces.start(found.start),
            pieces.end(found.tail_end - 1),
            format!("{not}{prefix}{}{group}", rule.replacement),
        )
    }

    fn matched<'r>(pieces: &Pieces<'_>, rule: &'r CallRule, at: usize) -> Option<CallMatch<'r>> {
        let name = Some(at).filter(|&index| pieces.is(index, rule.name))?;
        let group_end = Self::group_end(pieces, rule, name)?;
        let tail = rule
            .tails
            .iter()
            .find(|tail| pieces.matches(group_end, tail))?;
        let tail_end = group_end + tail.len();
        let start = Self::start(pieces, rule, name)?;
        let edges = ExpressionEdges::is_open_left(pieces, start)
            & ExpressionEdges::is_closed_right(pieces, tail_end);
        let fits = edges
            & Self::arguments_fit(pieces, rule, name, group_end)
            & pieces.is_plain(start, tail_end);
        fits.then_some(CallMatch {
            rule,
            start,
            name,
            group_end,
            tail_end,
        })
    }

    fn group_end(pieces: &Pieces<'_>, rule: &CallRule, name: usize) -> Option<usize> {
        let open = name + 1;
        let grouped = rule.groups.contains(&pieces.text(open)) & pieces.is_opener(open);
        let empty =
            rule.groups.contains(&CallRule::EMPTY_PARENTHESES) & pieces.matches(open, &["(", ")"]);
        let bare = rule.groups.contains(&CallRule::BARE);
        [
            grouped.then(|| pieces.after(open)),
            empty.then_some(open + 2),
            bare.then_some(open),
        ]
        .into_iter()
        .flatten()
        .next()
    }

    fn start(pieces: &Pieces<'_>, rule: &CallRule, name: usize) -> Option<usize> {
        let accessed =
            ExpressionEdges::is_access(pieces, name.wrapping_sub(1)) & pieces.touches(name);
        let receiver_last = name.wrapping_sub(2);
        let chain = ExpressionEdges::chain_start(pieces, receiver_last);
        let start = [name, chain][usize::from(rule.method)];
        let receiver = pieces.slice(start, name.wrapping_sub(1));
        let known = rule.receivers.is_empty() | rule.receivers.contains(&receiver);
        let shaped =
            (accessed == rule.method) & (!rule.method | pieces.is_value_end(receiver_last));
        (known & shaped).then_some(start)
    }

    fn arguments_fit(pieces: &Pieces<'_>, rule: &CallRule, name: usize, group_end: usize) -> bool {
        let from = name + 2;
        let to = group_end.saturating_sub(1).max(from);
        let commas = pieces.count_top(from, to, &[","]);
        let counted = (rule.arguments == 0) | (commas < rule.arguments);
        let forbidden = (from..to).any(|at| Self::is_forbidden(pieces, rule, at));
        counted & !forbidden
    }

    fn is_forbidden(pieces: &Pieces<'_>, rule: &CallRule, at: usize) -> bool {
        let text = pieces.text(at);
        let quoted = rule.forbidden.contains(&CallRule::CHARACTER) & text.starts_with('\'');
        rule.forbidden.contains(&text) | quoted
    }
}
