use super::super::duplication::ProjectSource;
use super::class_file::ClassFile;
use super::design_problem::DesignProblem;
use escape_hatch::EscapeHatch;
use javascript_types::JavascriptTypes;
use known_classes::KnownClasses;
use php_types::PhpTypes;
use python_types::PythonTypes;
use ruby_types::RubyTypes;
use typescript_types::TypescriptTypes;
use unfilled_field::UnfilledField;
use yard_names::YardNames;

mod escape_hatch;
mod field_value;
mod javascript_types;
mod known_classes;
mod method_parameters;
mod missing_type;
mod native_return;
mod php_types;
mod python_types;
mod return_value;
mod ruby_types;
mod tagged_method;
mod type_comments;
mod type_insertion;
mod type_tags;
mod typescript_types;
mod unfilled_field;
mod yard_names;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) use missing_type::MissingType;

pub(super) struct TypeRule;

impl TypeRule {
    pub(super) fn problems(classes: &[ClassFile], sources: &[ProjectSource]) -> Vec<DesignProblem> {
        let known = KnownClasses::read(classes, sources);
        let mut problems: Vec<DesignProblem> = Self::missing(classes, &known)
            .iter()
            .map(MissingType::problem)
            .collect();
        problems.extend(classes.iter().flat_map(EscapeHatch::problems));
        problems.extend(classes.iter().flat_map(UnfilledField::problems));
        problems.extend(
            classes
                .iter()
                .flat_map(|class| YardNames::problems(class, &known)),
        );
        problems
    }

    pub(super) fn missing_types(
        classes: &[ClassFile],
        sources: &[ProjectSource],
    ) -> Vec<MissingType> {
        let known = KnownClasses::read(classes, sources);
        Self::missing(classes, &known)
    }

    fn missing(classes: &[ClassFile], known: &KnownClasses) -> Vec<MissingType> {
        classes
            .iter()
            .flat_map(|class| Self::class_missing(class, known))
            .collect()
    }

    fn class_missing(class: &ClassFile, known: &KnownClasses) -> Vec<MissingType> {
        match class.extension {
            "js" => JavascriptTypes::missing(class, known),
            "ts" => TypescriptTypes::missing(class, known),
            "py" => PythonTypes::missing(class),
            "php" => PhpTypes::missing(class, known),
            "rb" => RubyTypes::missing(class, known),
            _ => Vec::new(),
        }
    }
}
