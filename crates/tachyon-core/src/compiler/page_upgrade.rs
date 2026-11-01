use super::css_rules::CssRules;
use super::css_tokens::CssTokens;
use super::foundation_rules::FoundationRules;
use super::page_composition::PageComposition;
use super::page_state::PageState;
use super::split_styles::SplitStyles;
use crate::template::TemplateFrontend;
use std::collections::BTreeSet;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct PageUpgrade;

impl PageUpgrade {
    pub(crate) fn markup_diagnostics(
        source: &str,
        source_path: &str,
        components: &BTreeSet<String>,
    ) -> Vec<Diagnostic> {
        PageState::strip_scripts(source, source_path).map_or_else(
            |failure| failure.diagnostics().to_vec(),
            |(template, state)| Self::composition(&template, &state, source_path, components),
        )
    }

    pub(crate) fn split_styles(source: &str) -> Option<SplitStyles> {
        let body = crate::without_bom(source);
        let offset = source.len() - body.len();
        let tokens = CssTokens::read(body).ok()?;
        let rules = CssRules::read(&tokens).ok()?;
        let starts: Vec<usize> = rules.iter().map(|rule| rule.start + offset).collect();
        let ends = starts.iter().skip(1).copied().chain([source.len()]);
        let chunks: Vec<(bool, &str)> = rules
            .iter()
            .zip(starts.iter().copied().zip(ends))
            .map(|(rule, (start, end))| (FoundationRules::accepts(rule), &source[start..end]))
            .collect();
        let first = starts.first().copied().unwrap_or(source.len());
        Some(SplitStyles::new(&source[..first], &chunks))
    }

    fn composition(
        template: &str,
        state: &str,
        source_path: &str,
        components: &BTreeSet<String>,
    ) -> Vec<Diagnostic> {
        TemplateFrontend::compile(template, source_path, components).map_or_else(
            |_| Vec::new(),
            |program| PageComposition::diagnostics(&program, state, source_path),
        )
    }
}
