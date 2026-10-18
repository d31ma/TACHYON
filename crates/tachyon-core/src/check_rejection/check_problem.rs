use super::ProblemEntry;
use serde::Serialize;
use tachyon_contracts::HandlerViolation;

#[derive(Debug, Serialize)]
pub(crate) struct CheckProblem {
    #[serde(rename = "type")]
    problem_type: &'static str,
    title: &'static str,
    status: u16,
    errors: Vec<ProblemEntry>,
    truncated: bool,
    reference: String,
}

impl CheckProblem {
    pub(crate) const STATUS: u16 = 422;
    pub(crate) const CONTENT_TYPE: &'static str = "application/problem+json";
    pub(crate) const MAX_ERRORS: usize = 32;
    pub(crate) const MAX_VIOLATIONS: usize = 1024;
    const TYPE: &'static str = "about:blank";
    const TITLE: &'static str = "Invalid input";

    pub(crate) fn new(violations: &[HandlerViolation], reference: &str) -> Self {
        let mut errors = Self::sorted_entries(violations);
        let truncated = errors.len() > Self::MAX_ERRORS;
        errors.truncate(Self::MAX_ERRORS);
        Self {
            problem_type: Self::TYPE,
            title: Self::TITLE,
            status: Self::STATUS,
            errors,
            truncated,
            reference: reference.to_owned(),
        }
    }

    pub(crate) fn accepts(violations: &[HandlerViolation]) -> bool {
        let counted = (1..=Self::MAX_VIOLATIONS).contains(&violations.len());
        let readable = Self::all_readable(violations);
        ![counted, readable].contains(&false)
    }

    fn all_readable(violations: &[HandlerViolation]) -> bool {
        violations
            .iter()
            .all(|violation| ProblemEntry::read(violation).is_some())
    }

    pub(crate) fn json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn sorted_entries(violations: &[HandlerViolation]) -> Vec<ProblemEntry> {
        let mut entries: Vec<ProblemEntry> =
            violations.iter().filter_map(ProblemEntry::read).collect();
        entries.sort();
        entries.dedup();
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::CheckProblem;
    use tachyon_contracts::HandlerViolation;

    fn violations(pairs: &[(&str, &str)]) -> Vec<HandlerViolation> {
        pairs
            .iter()
            .map(|(field, rule)| HandlerViolation::new(*field, *rule))
            .collect()
    }

    #[test]
    fn the_problem_body_follows_rfc_9457_with_a_reference() {
        let sent = violations(&[("price", "Min"), ("name", "NotEmpty")]);
        let problem = CheckProblem::new(&sent, "ref42");
        assert_eq!(
            problem.json(),
            r#"{"type":"about:blank","title":"Invalid input","status":422,"errors":[{"field":"name","rule":"NotEmpty"},{"field":"price","rule":"Min"}],"truncated":false,"reference":"ref42"}"#
        );
    }

    #[test]
    fn entries_sort_by_path_then_rule_and_repeats_collapse() {
        let sent = violations(&[
            ("items[10].name", "Required"),
            ("name", "Length"),
            ("items[2].name", "NotEmpty"),
            ("name", "NotEmpty"),
            ("", "Checked"),
            ("name", "Length"),
        ]);
        let reversed: Vec<HandlerViolation> = sent.iter().rev().cloned().collect();
        let first = CheckProblem::new(&sent, "a").json();
        let second = CheckProblem::new(&reversed, "a").json();
        assert_eq!(first, second);
        assert!(first.contains(
            r#""errors":[{"field":"","rule":"Checked"},{"field":"items[2].name","rule":"NotEmpty"},{"field":"items[10].name","rule":"Required"},{"field":"name","rule":"NotEmpty"},{"field":"name","rule":"Length"}]"#
        ), "{first}");
    }

    #[test]
    fn the_list_stops_at_thirty_two_entries_and_says_so() {
        let fields: Vec<String> = (0..40).map(|index| format!("items[{index}]")).collect();
        let sent: Vec<HandlerViolation> = fields
            .iter()
            .map(|field| HandlerViolation::new(field.as_str(), "Required"))
            .collect();
        let problem = CheckProblem::new(&sent, "a");
        let body: serde_json::Value = serde_json::from_str(&problem.json()).unwrap_or_default();
        let errors = body["errors"].as_array().cloned().unwrap_or_default();
        assert_eq!(errors.len(), CheckProblem::MAX_ERRORS);
        assert_eq!(errors[31]["field"], "items[31]");
        assert_eq!(body["truncated"], true);
        let exact = CheckProblem::new(&sent[..32], "a").json();
        assert!(exact.contains(r#""truncated":false"#), "{exact}");
    }

    #[test]
    fn only_well_formed_violation_lists_are_accepted() {
        assert!(CheckProblem::accepts(&violations(&[("name", "Required")])));
        assert!(!CheckProblem::accepts(&[]));
        assert!(!CheckProblem::accepts(&violations(&[("name", "notNull")])));
        assert!(!CheckProblem::accepts(&violations(&[("a b", "Required")])));
        assert!(!CheckProblem::accepts(&violations(&[(
            "secret value",
            "Required"
        )])));
        let many: Vec<HandlerViolation> = (0..=CheckProblem::MAX_VIOLATIONS)
            .map(|_| HandlerViolation::new("name", "Required"))
            .collect();
        assert!(!CheckProblem::accepts(&many));
        assert!(CheckProblem::accepts(&many[1..]));
    }
}
