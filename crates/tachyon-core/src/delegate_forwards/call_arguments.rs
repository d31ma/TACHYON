use super::{MarkerStrip, ParameterSplit, SourceLines};

pub(super) struct CallArguments;

impl CallArguments {
    const RUST_RECEIVERS: [&'static str; 4] = ["self", "&self", "&mut self", "mut self"];
    const RUST_SELF: &'static str = "self";
    const CSHARP_PASSING: [&'static str; 2] = ["ref", "out"];
    const DART_REQUIRED: &'static str = "required";
    const KOTLIN_SPREAD: &'static str = "vararg";
    const SWIFT_UNLABELED: &'static str = "_";
    const SWIFT_INOUT: &'static str = "inout";
    const GO_SPREAD: &'static str = "...";

    pub(super) fn joined(language: &str, parameters: &[String]) -> String {
        let arguments: Vec<String> = parameters
            .iter()
            .map(|parameter| MarkerStrip::plain(language, parameter))
            .flat_map(|parameter| Self::of(language, &parameter))
            .collect();
        arguments.join(", ")
    }

    fn of(language: &str, parameter: &str) -> Vec<String> {
        match language {
            "kt" => vec![Self::kotlin(parameter)],
            "swift" => vec![Self::swift(parameter)],
            "go" => vec![Self::go(parameter)],
            "rs" => Self::rust(parameter).into_iter().collect(),
            "fs" => vec![Self::name_before_colon(parameter)],
            "dart" => Self::dart(parameter),
            "cs" => vec![Self::csharp(parameter)],
            _ => vec![Self::last_name(parameter)],
        }
    }

    fn last_name(parameter: &str) -> String {
        let declared = parameter.split('=').next().unwrap_or(parameter).trim_end();
        let start = declared
            .rfind(|letter: char| !SourceLines::is_word_letter(letter))
            .map_or(0, |offset| offset + 1);
        declared[start..].to_owned()
    }

    fn name_before_colon(parameter: &str) -> String {
        let declared = parameter.split(':').next().unwrap_or(parameter);
        Self::last_name(declared)
    }

    fn csharp(parameter: &str) -> String {
        let first = parameter.split_whitespace().next().unwrap_or_default();
        let passing = Self::CSHARP_PASSING
            .iter()
            .find(|word| **word == first)
            .map(|word| format!("{word} "))
            .unwrap_or_default();
        format!("{passing}{}", Self::last_name(parameter))
    }

    fn kotlin(parameter: &str) -> String {
        let spread = parameter.trim_start().starts_with(Self::KOTLIN_SPREAD);
        let mark = ["", "*"][usize::from(spread)];
        format!("{mark}{}", Self::name_before_colon(parameter))
    }

    fn swift(parameter: &str) -> String {
        let declared = parameter.split(':').next().unwrap_or(parameter);
        let words: Vec<&str> = declared.split_whitespace().collect();
        let name = words.last().copied().unwrap_or_default();
        let label = words.first().copied().unwrap_or_default();
        let inout = ["", "&"][usize::from(parameter.contains(Self::SWIFT_INOUT))];
        let labels = [String::new(), format!("{label}: ")];
        let labeled = label != Self::SWIFT_UNLABELED;
        format!("{}{inout}{name}", labels[usize::from(labeled)])
    }

    fn go(parameter: &str) -> String {
        let name = parameter.split_whitespace().next().unwrap_or_default();
        let spread = parameter.contains(Self::GO_SPREAD);
        format!("{name}{}", ["", Self::GO_SPREAD][usize::from(spread)])
    }

    fn rust(parameter: &str) -> Option<String> {
        let name = Self::name_before_colon(parameter);
        let receiver = [
            Self::RUST_RECEIVERS.contains(&parameter.trim()),
            name == Self::RUST_SELF,
        ]
        .contains(&true);
        (!receiver).then_some(name)
    }

    fn dart(parameter: &str) -> Vec<String> {
        let named = parameter.starts_with('{');
        let grouped = parameter.starts_with(['{', '[']);
        let inside = parameter
            .get(1..parameter.len().saturating_sub(1))
            .unwrap_or_default();
        let inner = [parameter, inside][usize::from(grouped)];
        ParameterSplit::parts(inner, inner, 0..inner.len())
            .iter()
            .map(|part| Self::dart_argument(part, named))
            .collect()
    }

    fn dart_argument(part: &str, named: bool) -> String {
        let plain = part.trim_start_matches(Self::DART_REQUIRED).trim();
        let name = Self::last_name(plain);
        let labels = [String::new(), format!("{name}: ")];
        format!("{}{name}", labels[usize::from(named)])
    }
}
