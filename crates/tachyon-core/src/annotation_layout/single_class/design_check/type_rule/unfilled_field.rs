use super::super::braced_fields::BracedFields;
use super::super::class_file::ClassFile;
use super::super::design_problem::DesignProblem;
use super::super::design_rule::DesignRule;
use super::field_value::FieldValue;
use super::type_comments::TypeComments;

pub(super) struct UnfilledField;

impl UnfilledField {
    const JAVASCRIPT: &'static str = "js";
    const CLASS_WORD: &'static str = "class";
    const FIELD_MARKERS: [&'static str; 3] = ["Needs", "Setting", "Fake"];
    const CLASS_MARKERS: [&'static str; 3] = ["NeedsAll", "Needs", "Entity"];
    const MARK: char = '@';
    const NAME_JOIN: char = '_';
    const REASON: &'static str = "JavaScript has no `!` to say that a field is filled later, so the project's jsconfig.json turns off tsc's strictPropertyInitialization check and this check takes its place (ADR 0079).";

    pub(super) fn problems(class: &ClassFile) -> Vec<DesignProblem> {
        let javascript = class.extension == Self::JAVASCRIPT;
        let checked = javascript && !Self::fills_every_field(class);
        BracedFields::read(class)
            .into_iter()
            .filter(|_| checked)
            .filter(|&at| !FieldValue::has_value(class, at))
            .filter(|&at| !Self::marked(class, at, &Self::FIELD_MARKERS))
            .map(|at| Self::problem(class, at))
            .collect()
    }

    fn fills_every_field(class: &ClassFile) -> bool {
        class
            .find_word(Self::CLASS_WORD)
            .is_some_and(|at| Self::marked(class, at, &Self::CLASS_MARKERS))
    }

    fn marked(class: &ClassFile, at: usize, wanted: &[&str]) -> bool {
        Self::markers(class, at)
            .iter()
            .any(|name| wanted.contains(&name.as_str()))
    }

    fn markers(class: &ClassFile, at: usize) -> Vec<String> {
        let comments = TypeComments::above(class, at);
        let same_line = &class.text[comments.line..class.words.start(at)];
        format!("{}{same_line}", comments.markers)
            .split(Self::MARK)
            .skip(1)
            .map(Self::marker_name)
            .collect()
    }

    fn marker_name(text: &str) -> String {
        text.chars()
            .take_while(|character| character.is_alphanumeric() || (*character == Self::NAME_JOIN))
            .collect()
    }

    fn problem(class: &ClassFile, at: usize) -> DesignProblem {
        let name = class.words.text(at);
        class.problem(
            DesignRule::Types,
            at,
            format!(
                "The field `{name}` in {} has no starting value, and Tachyon does not fill it.",
                class.name
            ),
            format!(
                "Give it a starting value (`{name} = value`), or put `@Needs` above it when Tachyon should fill it. {}",
                Self::REASON
            ),
        )
    }
}
