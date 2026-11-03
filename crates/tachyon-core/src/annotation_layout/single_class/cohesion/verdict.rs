use super::scanned::Scanned;
use super::spelling::Spelling;
use super::words::Words;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeSet;
use tachyon_diagnostics::Diagnostic;

pub(super) struct Verdict<'r, 'a> {
    path: String,
    words: &'r Words<'a>,
    spelling: &'r Spelling,
    opt_out: &'static str,
    scanned: Vec<Scanned>,
    branching: BTreeSet<String>,
}

impl<'r, 'a> Verdict<'r, 'a> {
    const CODE: u16 = 1314;
    const TAC_OPT_OUT: &'static str = "`tac.cohesion.check` in client/manifest.json";
    const YON_OPT_OUT: &'static str = "`cohesion.check` in server/application.json";

    pub(super) fn new(
        path: &str,
        words: &'r Words<'a>,
        spelling: &'r Spelling,
        scanned: Vec<Scanned>,
    ) -> Self {
        let branching = Self::branching_names(&scanned);
        Self {
            path: path.to_owned(),
            words,
            spelling,
            opt_out: Self::opt_out_for(path),
            scanned,
            branching,
        }
    }

    pub(super) fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut found = self.public_problems();
        found.extend(self.private_problems());
        found.extend(self.helper_call_problems());
        found.sort_by_key(|problem| problem.spans.first().map(|span| span.start));
        found
    }

    fn opt_out_for(path: &str) -> &'static str {
        if path.starts_with("client/") {
            return Self::TAC_OPT_OUT;
        }
        Self::YON_OPT_OUT
    }

    fn branching_names(scanned: &[Scanned]) -> BTreeSet<String> {
        scanned
            .iter()
            .filter(|method| method.is_branching_helper())
            .map(|method| method.method.name.clone())
            .collect()
    }

    fn public_problems(&self) -> Vec<Diagnostic> {
        self.scanned
            .iter()
            .filter(|method| method.method.public & !method.findings.is_empty())
            .map(|method| self.public_problem(method))
            .collect()
    }

    fn private_problems(&self) -> Vec<Diagnostic> {
        self.scanned
            .iter()
            .filter(|method| !method.method.public & (method.decisions().len() > 1))
            .map(|method| self.private_problem(method))
            .collect()
    }

    fn helper_call_problems(&self) -> Vec<Diagnostic> {
        self.scanned
            .iter()
            .filter(|method| method.is_branching_helper())
            .flat_map(|method| self.calls_from(method))
            .collect()
    }

    fn calls_from(&self, method: &Scanned) -> Vec<Diagnostic> {
        method
            .method
            .body
            .clone()
            .filter(|&at| method.method.owns(at) & self.calls_branching_helper(method, at))
            .map(|at| self.helper_call_problem(method, at))
            .collect()
    }

    fn calls_branching_helper(&self, method: &Scanned, at: usize) -> bool {
        let name = self.words.text(at);
        let helper = self.branching.contains(name) & (name != method.method.name);
        let receiver = &method.method.receiver;
        let parentheses = self.spelling.calls_need_parentheses();
        helper & self.words.is_self_call(at, receiver, parentheses)
    }

    fn public_problem(&self, method: &Scanned) -> Diagnostic {
        let name = &method.method.name;
        let first = &method.findings[0];
        let message = format!(
            "Public method `{name}` holds control flow: `{}`. A public method only coordinates: it calls injected services and private methods, and it stores or returns their results.",
            method.words()
        );
        let help = format!(
            "Move the `{}` and the code that it controls into {}, and call that method from `{name}`. A private method holds at most one decision or loop. While you migrate, you can set {} to false (ADR 0063).",
            first.word,
            self.spelling.private_hint(),
            self.opt_out
        );
        self.problem(first.at, message, help)
    }

    fn private_problem(&self, method: &Scanned) -> Diagnostic {
        let name = &method.method.name;
        let decisions = method.decisions();
        let second = decisions[1];
        let message = format!(
            "Private method `{name}` holds {} decisions or loops: `{}`. A private method holds at most one.",
            decisions.len(),
            method.words()
        );
        let help = format!(
            "Split `{name}`: move the `{}` and the code that it controls into another private method, and call both methods from a public method (ADR 0063).",
            second.word
        );
        self.problem(second.at, message, help)
    }

    fn helper_call_problem(&self, method: &Scanned, at: usize) -> Diagnostic {
        let name = &method.method.name;
        let callee = self.words.text(at);
        let message = format!(
            "Private method `{name}` makes a decision and calls `{callee}`, which makes another decision."
        );
        let help = format!(
            "Call `{callee}` from a public method instead, and pass its result to `{name}`. Then each private method holds one decision (ADR 0063)."
        );
        self.problem(at, message, help)
    }

    fn problem(&self, at: usize, message: String, help: String) -> Diagnostic {
        let start = self.words.start(at);
        let end = start + self.words.text(at).len().max(1);
        diagnostic(
            Self::CODE,
            message,
            Some(help),
            source_span(&self.path, start, end),
        )
    }
}
