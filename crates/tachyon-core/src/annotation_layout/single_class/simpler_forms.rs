#![allow(clippy::needless_bitwise_bool)]

use super::rule_switch::RuleSwitch;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use form_files::FormFiles;
use form_scan::FormScan;
use form_suggestion::FormSuggestion;
use receiver_forms::ReceiverForms;
use std::fmt::Write as _;
use std::path::Path;

mod any_instead_of_filter;
mod block_text;
mod boolean_assignment;
mod boolean_branch;
mod boolean_choice;
mod boolean_compare;
mod boolean_return;
mod call_match;
mod call_rewrite;
mod call_rule;
mod code_language;
mod compound_assignment;
mod condition;
mod condition_context;
mod contains_instead_of_index;
mod dictionary_get;
mod double_negation;
mod else_after_exit;
mod expression_edges;
mod extra_return;
mod for_each;
mod form_files;
mod form_pattern;
mod form_scan;
mod form_suggestion;
mod if_part;
mod if_statement;
mod is_empty;
mod joined_if;
mod lookup_rule;
mod loop_shapes;
mod negated_equality;
mod piece;
mod pieces;
mod shape;
mod simpler_form;
mod size_rule;
mod total_loop;
mod total_shape;

mod receiver_forms;
#[cfg(test)]
mod receiver_runtime_tests;
mod receiver_scope;
#[cfg(test)]
mod receiver_tests;
#[cfg(test)]
mod tests;

pub(crate) use simpler_form::SimplerForm;

#[derive(Clone, Copy, Debug, Default)]
pub struct SimplerForms;

impl SimplerForms {
    const RULE: &'static str = "simplify";
    const RECEIVER_RULE: &'static str = "implicitmembers";

    #[must_use]
    pub fn report(project_root: &Path) -> String {
        let switch = Self::switch(project_root);
        let receiver_switch = Self::receiver_switch(project_root);
        let suggestions: Vec<FormSuggestion> = FormFiles::read(project_root)
            .iter()
            .flat_map(|(path, text)| Self::suggestions(path, text, switch, receiver_switch))
            .collect();
        Self::text(&suggestions)
    }

    pub(crate) fn edits(project_root: &Path, relative: &str, source: &str) -> Vec<SimplerForm> {
        let switch = Self::switch(project_root);
        let receivers = Self::receiver_switch(project_root);
        Self::forms(relative, source, switch, receivers)
    }

    fn forms(
        relative: &str,
        source: &str,
        switch: RuleSwitch,
        receivers: RuleSwitch,
    ) -> Vec<SimplerForm> {
        let wanted = switch.covers(Path::new(relative));
        let forms = FormScan::forms(Path::new(relative), source);
        let receiver_wanted = receivers.covers(Path::new(relative));
        let receiver_forms = receiver_wanted
            .then(|| ReceiverForms::find(Path::new(relative), source))
            .unwrap_or_default();
        let selected = forms.into_iter().filter(|_| wanted).collect();
        let selected_receivers = receiver_forms
            .into_iter()
            .filter(|_| receiver_wanted)
            .collect();
        FormScan::combine(selected, selected_receivers)
    }

    fn switch(project_root: &Path) -> RuleSwitch {
        Dir::open_ambient_dir(project_root, ambient_authority())
            .map_or(RuleSwitch::NONE, |project| {
                RuleSwitch::suggestions(&project, Self::RULE)
            })
    }

    fn receiver_switch(project_root: &Path) -> RuleSwitch {
        Dir::open_ambient_dir(project_root, ambient_authority())
            .map_or(RuleSwitch::NONE, |project| {
                RuleSwitch::suggestions_or(&project, Self::RECEIVER_RULE, false)
            })
    }

    fn suggestions(
        path: &str,
        text: &str,
        switch: RuleSwitch,
        receivers: RuleSwitch,
    ) -> Vec<FormSuggestion> {
        Self::forms(path, text, switch, receivers)
            .iter()
            .map(|form| FormSuggestion::new(path, text, form))
            .collect()
    }

    fn text(suggestions: &[FormSuggestion]) -> String {
        let mut text = String::new();
        for suggestion in suggestions {
            let _ = writeln!(text, "{suggestion}");
        }
        text + &Self::summary(suggestions.len())
    }

    fn summary(count: usize) -> String {
        if count == 0 {
            return String::new();
        }
        format!(
            "{count} simpler form(s) suggested. Run 'ty migrate fix --write' to apply them, or set tac.simplify.suggest / simplify.suggest to false to hide them. Receiver suggestions are controlled separately by tac.implicitmembers.suggest / implicitmembers.suggest.\n"
        )
    }
}
