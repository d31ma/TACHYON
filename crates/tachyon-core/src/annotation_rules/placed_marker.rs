use super::scan::Messages;
use super::{AnnotationRules, Arguments, Realm, Rule, Target};

pub(crate) struct PlacedMarker<'a> {
    pub(crate) name: &'a str,
    pub(crate) target: Target,
    pub(crate) has_arguments: bool,
    pub(crate) neighbours: &'a [String],
    pub(crate) class_markers: &'a [String],
}

impl PlacedMarker<'_> {
    pub(crate) fn problem(&self, language: &str, realm: Realm) -> Option<(u16, String)> {
        let rule = AnnotationRules::rules().find(|rule| rule.name == self.name)?;
        let message = self.message(rule, language, realm)?;
        Some((rule.code, message))
    }

    fn message(&self, rule: &Rule, language: &str, realm: Realm) -> Option<String> {
        Self::retired(rule, language)
            .or_else(|| Self::unavailable(rule, language, realm))
            .or_else(|| self.misplaced(rule, language))
            .or_else(|| self.argued(rule, language))
            .or_else(|| self.repeated(rule, language))
            .or_else(|| self.partnered(rule, language))
            .or_else(|| self.clashing(rule, language))
            .or_else(|| self.outside(rule, language))
    }

    fn retired(rule: &Rule, language: &str) -> Option<String> {
        rule.is_retired()
            .then(|| Messages::retired_text(rule, language))
    }

    fn unavailable(rule: &Rule, language: &str, realm: Realm) -> Option<String> {
        let checked = realm.languages().contains(&language);
        let refused = [checked, rule.allows(language, realm)] == [true, false];
        refused.then(|| Messages::unavailable(rule, language, realm))
    }

    fn misplaced(&self, rule: &Rule, language: &str) -> Option<String> {
        let wrong = !rule.targets.contains(&self.target);
        wrong.then(|| Messages::wrong_target(rule, language, self.target))
    }

    fn argued(&self, rule: &Rule, language: &str) -> Option<String> {
        match (rule.arguments, self.has_arguments) {
            (Arguments::None, true) => Some(Messages::no_arguments_text(rule, language)),
            (Arguments::Required, false) => Some(Messages::needs_arguments(rule, language)),
            _ => None,
        }
    }

    fn repeated(&self, rule: &Rule, language: &str) -> Option<String> {
        let twice = self.neighbours.iter().any(|name| name == rule.name);
        twice.then(|| Messages::twice(rule, language, self.target))
    }

    fn partnered(&self, rule: &Rule, language: &str) -> Option<String> {
        let missing = Self::lacks_any(rule.with, self.neighbours);
        missing.then(|| Messages::without_partner(rule, language, self.target))
    }

    fn clashing(&self, rule: &Rule, language: &str) -> Option<String> {
        let other = rule
            .clashes
            .iter()
            .find(|name| self.neighbours.iter().any(|neighbour| neighbour == *name))?;
        let other = AnnotationRules::rules().find(|candidate| candidate.name == *other)?;
        Some(Messages::clash(rule, other, language, self.target))
    }

    fn outside(&self, rule: &Rule, language: &str) -> Option<String> {
        let missing = Self::lacks_any(rule.class_needs, self.class_markers);
        missing.then(|| Messages::outside_class(rule, language))
    }

    fn lacks_any(wanted: &[&str], present: &[String]) -> bool {
        let needed = !wanted.is_empty();
        let found = wanted
            .iter()
            .any(|name| present.iter().any(|written| written == name));
        [needed, found] == [true, false]
    }
}
