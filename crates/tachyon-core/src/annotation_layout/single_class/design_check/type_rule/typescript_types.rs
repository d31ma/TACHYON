use super::super::braced_fields::BracedFields;
use super::super::class_file::ClassFile;
use super::field_value::FieldValue;
use super::known_classes::KnownClasses;
use super::method_parameters::MethodParameters;
use super::missing_type::MissingType;
use super::tagged_method::TaggedMethod;
use super::type_insertion::TypeInsertion;

pub(super) struct TypescriptTypes;

impl TypescriptTypes {
    const TYPED: [&'static str; 2] = [":", "="];
    const UNKNOWN: &'static str = "T";
    const DEFINITE: &'static str = "!";
    const PARAMETER: &'static str = "NAME: T";

    pub(super) fn missing(class: &ClassFile, known: &KnownClasses) -> Vec<MissingType> {
        let mut missing = TaggedMethod::each(class, |method| {
            TaggedMethod::new(class, method).parameters(Self::PARAMETER, |_, part| {
                MethodParameters::has_base_word(class, part, &Self::TYPED)
            })
        });
        missing.extend(
            BracedFields::read(class)
                .into_iter()
                .filter(|&at| !Self::typed_field(class, at))
                .map(|at| Self::field(class, known, at)),
        );
        missing
    }

    fn typed_field(class: &ClassFile, at: usize) -> bool {
        let next = FieldValue::after_name(class, at);
        Self::TYPED.contains(&class.words.text(next))
    }

    fn field(class: &ClassFile, known: &KnownClasses, at: usize) -> MissingType {
        let name = class.words.text(at);
        let bare = FieldValue::after_name(class, at) == at + 1;
        let injected = known.injected(name).filter(|_| bare);
        let annotation = injected.map_or_else(
            || format!(": {}", Self::UNKNOWN),
            |type_name| format!("{}: {type_name}", Self::DEFINITE),
        );
        let insertion = injected.map(|_| TypeInsertion {
            at: class.words.start(at) + name.len(),
            text: annotation.clone(),
        });
        MissingType::of(
            class,
            at,
            format!("field `{name}`"),
            format!("{name}{annotation}"),
        )
        .inserting(insertion)
    }
}
