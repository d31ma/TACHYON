use super::super::super::cohesion::Method;
use super::super::braced_fields::BracedFields;
use super::super::class_file::ClassFile;
use super::super::design_problem::DesignProblem;
use super::super::design_rule::DesignRule;
use super::field_value::FieldValue;
use super::known_classes::KnownClasses;
use super::type_comments::TypeComments;

pub(super) struct UnfilledField;

impl UnfilledField {
    const JAVASCRIPT: &'static str = "js";
    const CLASS_WORD: &'static str = "class";
    const FIELD_MARKERS: [&'static str; 3] = ["Needs", "Setting", "Fake"];
    const CLASS_MARKERS: [&'static str; 3] = ["NeedsAll", "Needs", "Entity"];
    const START_MARKERS: [&'static str; 1] = ["OnStart"];
    const CONSTRUCTOR: &'static str = "constructor";
    const ASSIGNMENT: [&'static str; 3] = ["this", ".", "="];
    const MARK: char = '@';
    const NAME_JOIN: char = '_';
    const REASON: &'static str = "JavaScript has no `!` to say that a field is filled later, so the project's jsconfig.json turns off tsc's strictPropertyInitialization check and this check takes its place (ADR 0079).";

    pub(super) fn problems(class: &ClassFile) -> Vec<DesignProblem> {
        let checked = Self::is_checked(class);
        BracedFields::read(class)
            .into_iter()
            .filter(|_| checked)
            .filter(|&at| !FieldValue::has_value(class, at))
            .filter(|&at| !Self::marked(class, at, &Self::FIELD_MARKERS))
            .filter(|&at| !KnownClasses::is_framework_field(class.words.text(at)))
            .filter(|&at| !Self::set_at_start(class, class.words.text(at)))
            .map(|at| Self::problem(class, at))
            .collect()
    }

    fn is_checked(class: &ClassFile) -> bool {
        let javascript = class.extension == Self::JAVASCRIPT;
        javascript && !Self::fills_every_field(class)
    }

    fn fills_every_field(class: &ClassFile) -> bool {
        class
            .find_word(Self::CLASS_WORD)
            .is_some_and(|at| Self::marked(class, at, &Self::CLASS_MARKERS))
    }

    fn set_at_start(class: &ClassFile, name: &str) -> bool {
        class
            .methods
            .iter()
            .filter(|method| Self::starts_the_object(class, method))
            .any(|method| Self::assigns(class, method, name))
    }

    fn starts_the_object(class: &ClassFile, method: &Method) -> bool {
        let constructor = method.name == Self::CONSTRUCTOR;
        constructor || Self::marked(class, method.head, &Self::START_MARKERS)
    }

    fn assigns(class: &ClassFile, method: &Method, name: &str) -> bool {
        let [this, dot, equals] = Self::ASSIGNMENT;
        let wanted = [this, dot, name, equals];
        method.body.clone().any(|at| {
            wanted
                .iter()
                .enumerate()
                .all(|(step, word)| class.words.text(at + step) == *word)
        })
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
            .take_while(|character| Self::is_name_character(*character))
            .collect()
    }

    fn is_name_character(character: char) -> bool {
        character.is_alphanumeric() || (character == Self::NAME_JOIN)
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
                "Give it a starting value (`{name} = value`) or set it in the constructor or an `@OnStart` method; put `@Needs` above it when Tachyon should fill it. {}",
                Self::REASON
            ),
        )
    }
}
