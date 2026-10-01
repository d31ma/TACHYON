#![allow(clippy::needless_bitwise_bool)]

use super::super::super::Input;
use super::super::Tokens;
use super::super::cohesion::Words;
use super::any_instead_of_filter::AnyInsteadOfFilter;
use super::boolean_assignment::BooleanAssignment;
use super::boolean_choice::BooleanChoice;
use super::boolean_compare::BooleanCompare;
use super::boolean_return::BooleanReturn;
use super::code_language::CodeLanguage;
use super::compound_assignment::CompoundAssignment;
use super::condition_context::ConditionContext;
use super::contains_instead_of_index::ContainsInsteadOfIndex;
use super::dictionary_get::DictionaryGet;
use super::double_negation::DoubleNegation;
use super::else_after_exit::ElseAfterExit;
use super::extra_return::ExtraReturn;
use super::for_each::ForEach;
use super::if_statement::IfStatement;
use super::is_empty::IsEmpty;
use super::joined_if::JoinedIf;
use super::negated_equality::NegatedEquality;
use super::pieces::Pieces;
use super::simpler_form::SimplerForm;
use super::total_loop::TotalLoop;
use std::path::Path;

pub(super) struct FormScan;

impl FormScan {
    const MAX_SOURCE_BYTES: usize = 256 * 1024;
    const MINIFIED: &'static str = ".min.";

    pub(super) fn combine(
        found: Vec<SimplerForm>,
        receivers: Vec<SimplerForm>,
    ) -> Vec<SimplerForm> {
        Self::separate(found.into_iter().chain(receivers).collect())
    }

    pub(super) fn forms(path: &Path, source: &str) -> Vec<SimplerForm> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let readable = (source.len() <= Self::MAX_SOURCE_BYTES) & !name.contains(Self::MINIFIED);
        let found = CodeLanguage::of(extension)
            .filter(|_| readable)
            .and_then(|language| Self::scan(path, source, language))
            .unwrap_or_default();
        Self::separate(found)
    }

    fn scan(path: &Path, source: &str, language: CodeLanguage) -> Option<Vec<SimplerForm>> {
        let input = Input::new(path, source);
        let tokens = Tokens::read(&input).ok()?;
        tokens.check_complete(&input).ok()?;
        let words = Words::read(&tokens, source);
        let pieces = Pieces::read(&words, source, language);
        let statements: Vec<IfStatement> = (0..pieces.len())
            .filter_map(|at| IfStatement::read(&pieces, at))
            .collect();
        let context = ConditionContext::new(&statements);
        Some(
            [
                Self::each_statement(&pieces, &statements, BooleanReturn::form),
                Self::each_statement(&pieces, &statements, BooleanAssignment::form),
                BooleanCompare::find(&pieces),
                BooleanChoice::find(&pieces, &statements),
                AnyInsteadOfFilter::find(&pieces),
                ContainsInsteadOfIndex::find(&pieces),
                IsEmpty::find(&pieces),
                DictionaryGet::find(&pieces, &statements),
                JoinedIf::find(&pieces, &statements),
                ElseAfterExit::find(&pieces, &statements),
                DoubleNegation::find(&pieces, &context),
                NegatedEquality::find(&pieces, &context),
                ExtraReturn::find(&pieces),
                CompoundAssignment::find(&pieces),
                ForEach::find(&pieces),
                TotalLoop::find(&pieces),
            ]
            .concat(),
        )
    }

    fn each_statement(
        pieces: &Pieces<'_>,
        statements: &[IfStatement],
        form: fn(&Pieces<'_>, &IfStatement) -> Option<SimplerForm>,
    ) -> Vec<SimplerForm> {
        statements
            .iter()
            .filter_map(|statement| form(pieces, statement))
            .collect()
    }

    fn separate(mut found: Vec<SimplerForm>) -> Vec<SimplerForm> {
        found.sort_by_key(|form| (form.start, std::cmp::Reverse(form.end)));
        let mut kept: Vec<SimplerForm> = Vec::new();
        for form in found {
            Self::keep_if_separate(&mut kept, form);
        }
        kept
    }

    fn keep_if_separate(kept: &mut Vec<SimplerForm>, form: SimplerForm) {
        let separate = kept.iter().all(|other| !other.overlaps(&form));
        kept.extend(separate.then_some(form));
    }
}
