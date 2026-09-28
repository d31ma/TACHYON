#![allow(clippy::needless_bitwise_bool)]

use super::class_file::ClassFile;
use super::condition_spans::ConditionSpans;
use super::constant_spelling::ConstantSpelling;
use super::design_problem::DesignProblem;
use super::design_rule::DesignRule;
use super::literal::Literal;
use super::literal_reader::LiteralReader;
use super::named_value::NamedValue;
use std::collections::BTreeMap;

pub(super) struct ConstantRule;

impl ConstantRule {
    const DATA_FOLDERS: [&'static str; 2] = ["models", "entities"];
    const KINDS: [&'static str; 2] = ["text", "number"];

    pub(super) fn problems(classes: &[ClassFile]) -> Vec<DesignProblem> {
        classes
            .iter()
            .filter(|class| !Self::DATA_FOLDERS.contains(&class.folder))
            .flat_map(Self::class_problems)
            .collect()
    }

    fn class_problems(class: &ClassFile) -> Vec<DesignProblem> {
        let named = NamedValue::new(class);
        let conditions = ConditionSpans::read(class);
        let loose: Vec<Literal> = LiteralReader::read(class)
            .into_iter()
            .filter(|literal| !named.is_named(literal.at))
            .collect();
        let groups = Self::grouped(loose);
        groups
            .iter()
            .filter_map(|((number, _), places)| Self::problem(class, *number, places, &conditions))
            .collect()
    }

    fn grouped(literals: Vec<Literal>) -> BTreeMap<(bool, String), Vec<usize>> {
        let mut groups: BTreeMap<(bool, String), Vec<usize>> = BTreeMap::new();
        for literal in literals {
            groups
                .entry((literal.number, literal.value))
                .or_default()
                .push(literal.at);
        }
        groups
    }

    fn problem(
        class: &ClassFile,
        number: bool,
        places: &[usize],
        conditions: &ConditionSpans,
    ) -> Option<DesignProblem> {
        let compared = places
            .iter()
            .copied()
            .find(|&at| number & conditions.holds(class, at));
        let repeated = (places.len() > 1).then(|| places[0]);
        let condition_problem = compared.map(|at| Self::condition_problem(class, at));
        let repeated_problem = repeated.map(|at| Self::repeated_problem(class, number, at, places));
        condition_problem.or(repeated_problem)
    }

    fn condition_problem(class: &ClassFile, at: usize) -> DesignProblem {
        let literal = Self::shown(class, at);
        let line = class.line(class.words.start(at));
        let message = format!(
            "The number {literal} in a condition in {} (line {line}) has no name.",
            class.name
        );
        class.problem(
            DesignRule::Constants,
            at,
            message,
            Self::help(class, at, true),
        )
    }

    fn repeated_problem(
        class: &ClassFile,
        number: bool,
        at: usize,
        places: &[usize],
    ) -> DesignProblem {
        let lines: Vec<String> = places
            .iter()
            .map(|&place| class.line(class.words.start(place)).to_string())
            .collect();
        let message = format!(
            "The {} {} appears {} times in {} (lines {}).",
            Self::KINDS[usize::from(number)],
            Self::shown(class, at),
            places.len(),
            class.name,
            lines.join(", ")
        );
        class.problem(
            DesignRule::Constants,
            at,
            message,
            Self::help(class, at, number),
        )
    }

    fn shown(class: &ClassFile, at: usize) -> String {
        let words = &class.words;
        let signed = usize::from(words.text(at.wrapping_sub(1)) == "-");
        let fraction = (words.text(at + 1) == ".")
            & words.touches_previous(at + 1)
            & words.touches_previous(at + 2)
            & Self::starts_with_digit(words.text(at + 2));
        let last = at + 2 * usize::from(fraction);
        let start = words.start(at - signed);
        let end = words.start(last) + words.text(last).len();
        class.text.get(start..end).unwrap_or("").to_owned()
    }

    fn starts_with_digit(text: &str) -> bool {
        text.starts_with(|first: char| first.is_ascii_digit())
    }

    fn help(class: &ClassFile, at: usize, number: bool) -> String {
        let example =
            ConstantSpelling::example(class.extension, class.name, &Self::shown(class, at), number);
        format!(
            "Give the value a name that says what it means: declare a class constant, such as `{example}`, and use the name in each place."
        )
    }
}
