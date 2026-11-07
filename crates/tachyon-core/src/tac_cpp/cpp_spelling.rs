use crate::annotation_rules::AnnotationRules;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CppSpelling {
    Exact,
    Cased(&'static str),
    OwnNamespace,
    Vendor,
    Standard,
    Unknown,
}

impl CppSpelling {
    const STANDARD: [&'static str; 10] = [
        "noreturn",
        "carries_dependency",
        "deprecated",
        "fallthrough",
        "nodiscard",
        "maybe_unused",
        "likely",
        "unlikely",
        "no_unique_address",
        "assume",
    ];
    const OWN_NAMESPACES: [&'static str; 3] = ["tachyon", "tac", "yon"];

    pub(crate) fn of(name: &str) -> Self {
        let namespace = name.rsplit_once("::").map(|(namespace, _)| namespace);
        let own = namespace.is_some_and(|namespace| Self::OWN_NAMESPACES.contains(&namespace));
        let standard = Self::STANDARD.contains(&name);
        let rule = AnnotationRules::find(name).map(|rule| rule.name);
        Self::classify([namespace.is_some(), own, standard], rule, name)
    }

    pub(crate) fn is_standard(name: &str) -> bool {
        Self::STANDARD.contains(&name)
    }

    fn classify(facts: [bool; 3], rule: Option<&'static str>, name: &str) -> Self {
        match (facts, rule) {
            ([true, true, _], _) => Self::OwnNamespace,
            ([true, false, _], _) => Self::Vendor,
            ([false, _, true], _) => Self::Standard,
            ([false, _, false], None) => Self::Unknown,
            ([false, _, false], Some(found)) if found == name => Self::Exact,
            ([false, _, false], Some(found)) => Self::Cased(found),
        }
    }
}
