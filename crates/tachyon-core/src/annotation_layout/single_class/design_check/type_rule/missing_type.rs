use super::super::class_file::ClassFile;
use super::super::design_problem::DesignProblem;
use super::super::design_rule::DesignRule;
use super::type_insertion::TypeInsertion;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MissingType {
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) insertion: Option<TypeInsertion>,
    start: usize,
    end: usize,
    owner: String,
    member: String,
    example: String,
    extension: String,
}

impl MissingType {
    const PLACES: [(&'static str, &'static str); 2] = [
        ("js", "to the JSDoc comment (`/** ... */`) above it"),
        ("rb", "as a comment line above it"),
    ];
    const DECLARATION: &'static str = "to its declaration";
    const UNKNOWN: &'static str = " Write the real type in place of `T`.";

    pub(super) fn of(class: &ClassFile, at: usize, member: String, example: String) -> Self {
        let start = class.words.start(at);
        Self {
            path: class.path.to_owned(),
            line: class.line(start),
            insertion: None,
            start,
            end: start + class.words.text(at).len(),
            owner: class.name.to_owned(),
            member,
            example,
            extension: class.extension.to_owned(),
        }
    }

    pub(super) fn inserting(self, insertion: Option<TypeInsertion>) -> Self {
        Self { insertion, ..self }
    }

    pub(crate) fn message(&self) -> String {
        format!("The {} in {} has no type.", self.member, self.owner)
    }

    pub(crate) fn help(&self) -> String {
        let unknown = [Self::UNKNOWN, ""][usize::from(self.insertion.is_some())];
        format!(
            "Add `{}` {}.{unknown} Every field, parameter and return value states its type; where the language has no type syntax, a type comment carries it (ADR 0079). `ty migrate fix --write` adds the types it can know and lists the rest.",
            self.example,
            self.place()
        )
    }

    pub(super) fn problem(&self) -> DesignProblem {
        DesignProblem {
            rule: DesignRule::Types,
            path: self.path.clone(),
            start: self.start,
            end: self.end,
            line: self.line,
            message: self.message(),
            help: self.help(),
        }
    }

    fn place(&self) -> &'static str {
        Self::PLACES
            .iter()
            .find(|(language, _)| *language == self.extension)
            .map_or(Self::DECLARATION, |(_, place)| place)
    }
}
