use super::super::super::cohesion::Method;
use super::super::class_file::ClassFile;
use super::super::line_fields::LineFields;
use super::field_value::FieldValue;
use super::method_parameters::MethodParameters;
use super::missing_type::MissingType;
use super::native_return::NativeReturn;
use super::tagged_method::TaggedMethod;

pub(super) struct PythonTypes;

impl PythonTypes {
    const COLON: [&'static str; 1] = [":"];
    const ARROW: &'static str = "->";
    const NEWLINE: char = '\n';
    const PARAMETER: &'static str = "NAME: T";
    const DEFINITION: &'static str = "def";
    const RETURN: NativeReturn = NativeReturn {
        typed: Self::has_arrow,
        nothing: " -> None",
        unknown: " -> T",
    };

    pub(super) fn missing(class: &ClassFile) -> Vec<MissingType> {
        let mut missing = TaggedMethod::each(class, |method| Self::method_missing(class, method));
        missing.extend(
            LineFields::read(class)
                .into_iter()
                .filter(|&at| Self::at_member_level(class, at))
                .filter(|&at| !Self::typed_field(class, at))
                .map(|at| Self::field(class, at)),
        );
        missing
    }

    fn method_missing(class: &ClassFile, method: &Method) -> Vec<MissingType> {
        let mut missing = TaggedMethod::new(class, method)
            .parameters(Self::PARAMETER, |_, part| {
                MethodParameters::has_base_word(class, part, &Self::COLON)
            });
        missing.extend(Self::RETURN.missing(class, method));
        missing
    }

    fn at_member_level(class: &ClassFile, at: usize) -> bool {
        let member_column = Self::member_column(class).unwrap_or(usize::MAX);
        class.words.column(at) <= member_column
    }

    fn member_column(class: &ClassFile) -> Option<usize> {
        (0..class.words.len())
            .filter(|&at| class.words.text(at) == Self::DEFINITION)
            .map(|at| class.words.column(class.index.line_begin(at)))
            .min()
    }

    fn has_arrow(text: &str, close: usize) -> bool {
        let rest = &text[close..];
        let line_end = rest.find(Self::NEWLINE).unwrap_or(rest.len());
        rest[..line_end].contains(Self::ARROW)
    }

    fn typed_field(class: &ClassFile, at: usize) -> bool {
        let typed = Self::COLON.contains(&class.words.text(at + 1));
        typed | FieldValue::is_literal(class, at)
    }

    fn field(class: &ClassFile, at: usize) -> MissingType {
        let name = class.words.text(at);
        MissingType::of(
            class,
            at,
            format!("field `{name}`"),
            format!("{name}: T = ..."),
        )
    }
}
