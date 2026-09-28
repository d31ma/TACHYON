use super::ended::EndedMembers;
use super::finding::Finding;
use super::method::Method;
use super::spelling::Spelling;
use super::words::Words;
use std::ops::Range;

pub(super) struct BodyScan<'r, 'a> {
    words: &'r Words<'a>,
    spelling: &'r Spelling,
}

impl<'r, 'a> BodyScan<'r, 'a> {
    const LOOPS: &'static [&'static str] = &["for", "foreach", "while", "until", "loop"];
    const CONDITIONS: &'static [&'static str] = &[
        "if", "elif", "elsif", "elseif", "unless", "while", "until", "guard",
    ];
    const CHAINED: &'static [&'static str] = &["elif", "elsif", "elseif"];
    const CONDITION_ENDS: &'static [&'static str] = &["{", ":", "then", "=>", "do", "else"];

    pub(super) fn new(words: &'r Words<'a>, spelling: &'r Spelling) -> Self {
        Self { words, spelling }
    }

    pub(super) fn findings(&self, method: &Method) -> Vec<Finding> {
        let body = method.body.clone();
        let conditions = self.conditions(body.clone());
        let mut found = self.plain_findings(body.clone(), &conditions);
        let iterators = self.iterator_findings(body, &found);
        found.extend(iterators);
        found.retain(|finding| method.owns(finding.at));
        found.sort_by_key(|finding| finding.at);
        found
    }

    fn conditions(&self, body: Range<usize>) -> Vec<Range<usize>> {
        body.filter(|&at| self.opens_condition(at))
            .map(|at| self.condition(at))
            .collect()
    }

    fn opens_condition(&self, at: usize) -> bool {
        let text = self.words.text(at);
        Self::CONDITIONS.contains(&text)
            & self.spelling.is_keyword(text)
            & !self.words.after_member_access(at)
    }

    fn condition(&self, at: usize) -> Range<usize> {
        let open = at + 1;
        let grouped = (self.words.text(open) == "(") & self.spelling.groups_conditions();
        if grouped {
            return open..self.words.after_group(open);
        }
        open..self.condition_end(at)
    }

    fn condition_end(&self, at: usize) -> usize {
        let depth = self.words.depth(at);
        (at + 1..self.words.len())
            .find(|&index| self.ends_condition(index, depth))
            .unwrap_or(self.words.len())
    }

    fn ends_condition(&self, at: usize, depth: usize) -> bool {
        let closes = self.words.depth(at) < depth;
        let marker =
            (self.words.depth(at) == depth) & Self::CONDITION_ENDS.contains(&self.words.text(at));
        closes | marker | self.words.line_start(at)
    }

    fn plain_findings(&self, body: Range<usize>, conditions: &[Range<usize>]) -> Vec<Finding> {
        body.flat_map(|at| self.plain_findings_at(at, conditions))
            .collect()
    }

    fn plain_findings_at(&self, at: usize, conditions: &[Range<usize>]) -> Vec<Finding> {
        [
            self.keyword(at),
            self.short_circuit(at, conditions),
            self.ternary(at),
            self.query(at),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn keyword(&self, at: usize) -> Option<Finding> {
        let text = self.words.text(at);
        let placed = !self.spelling.needs_line_start(text) | self.words.line_start(at);
        let found = self.spelling.is_keyword(text) & placed & !self.words.after_member_access(at);
        let chained = Self::CHAINED.contains(&text)
            | ((text == "if") & (self.words.text(at.wrapping_sub(1)) == "else"));
        let counts = Spelling::counts(text) & !chained;
        found.then(|| Self::finding(at, text, counts, !Self::LOOPS.contains(&text)))
    }

    fn short_circuit(&self, at: usize, conditions: &[Range<usize>]) -> Option<Finding> {
        let pair = self.words.pair(at);
        let found = self.spelling.is_pair(pair) & self.words.value_before(at);
        let in_condition = conditions.iter().any(|condition| condition.contains(&at));
        found.then(|| Self::finding(at, pair, !in_condition, true))
    }

    fn ternary(&self, at: usize) -> Option<Finding> {
        let found =
            self.spelling.has_ternary() & (self.words.text(at) == "?") & self.words.spaced(at);
        found.then(|| Self::finding(at, "? :", true, true))
    }

    fn query(&self, at: usize) -> Option<Finding> {
        let from = (self.words.text(at) == "from") & (self.words.text(at + 2) == "in");
        let found = self.spelling.has_query_syntax() & from & !self.words.after_member_access(at);
        found.then(|| Self::finding(at, "from … in", true, false))
    }

    fn iterator_findings(&self, body: Range<usize>, plain: &[Finding]) -> Vec<Finding> {
        body.filter(|&at| self.iterates_with_logic(at, plain))
            .map(|at| Self::finding(at, self.words.text(at), true, false))
            .collect()
    }

    fn iterates_with_logic(&self, at: usize, plain: &[Finding]) -> bool {
        let extent = at + 1..self.call_end(at);
        let holds_logic = plain.iter().any(|found| extent.contains(&found.at));
        Spelling::is_iterator(self.words.text(at)) & self.has_lambda(at) & holds_logic
    }

    fn call_end(&self, at: usize) -> usize {
        let after_arguments = self.after_arguments(at + 1);
        self.trailing_block_end(after_arguments)
    }

    fn after_arguments(&self, open: usize) -> usize {
        if self.words.text(open) == "(" {
            return self.words.after_group(open);
        }
        open
    }

    fn trailing_block_end(&self, at: usize) -> usize {
        let opens = self.spelling.opens_trailing_block(self.words.text(at));
        match (opens, self.words.text(at)) {
            (true, "do") => EndedMembers::block_end(self.words, at),
            (true, _) => self.words.after_group(at),
            _ => at,
        }
    }

    fn has_lambda(&self, at: usize) -> bool {
        let open = at + 1;
        let after_arguments = self.words.after_group(open);
        let parenthesis = self.words.text(open) == "(";
        let trailing = self.spelling.opens_trailing_block(self.words.text(open))
            | (parenthesis
                & self
                    .spelling
                    .opens_trailing_block(self.words.text(after_arguments)));
        let argument = parenthesis & self.lambda_in(self.words.inside(open));
        trailing | argument
    }

    fn lambda_in(&self, arguments: Range<usize>) -> bool {
        arguments.clone().any(|at| {
            self.spelling.is_lambda_mark(self.words.text(at))
                | self.spelling.is_lambda_mark(self.words.pair(at))
        })
    }

    fn finding(at: usize, word: &str, counts: bool, branches: bool) -> Finding {
        let counted = counts & Spelling::counts(word);
        Finding {
            at,
            word: word.to_owned(),
            counts: counted,
            branches: branches & counted,
        }
    }
}
