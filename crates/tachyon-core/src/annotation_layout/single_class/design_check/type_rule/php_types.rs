use super::super::super::cohesion::Method;
use super::super::braced_fields::BracedFields;
use super::super::class_file::ClassFile;
use super::super::name_picker::NamePicker;
use super::field_value::FieldValue;
use super::known_classes::KnownClasses;
use super::missing_type::MissingType;
use super::native_return::NativeReturn;
use super::tagged_method::TaggedMethod;
use super::type_insertion::TypeInsertion;
use std::ops::Range;

pub(super) struct PhpTypes;

impl PhpTypes {
    const DOLLAR: &'static str = "$";
    const CONSTANT: &'static str = "const";
    const COLON: char = ':';
    const HEADER_ENDS: [char; 2] = ['{', ';'];
    const UNKNOWN: &'static str = "T";
    const PARAMETER: &'static str = "T $NAME";
    const RETURN: NativeReturn = NativeReturn {
        typed: Self::has_type,
        nothing: ": void",
        unknown: ": T",
    };
    const MODIFIERS: [&'static str; 8] = [
        "public",
        "private",
        "protected",
        "readonly",
        "static",
        "var",
        "final",
        "abstract",
    ];

    pub(super) fn missing(class: &ClassFile, known: &KnownClasses) -> Vec<MissingType> {
        let mut missing = TaggedMethod::each(class, |method| Self::method_missing(class, method));
        missing.extend(
            BracedFields::read(class)
                .into_iter()
                .filter(|&at| !Self::is_constant(class, at))
                .filter(|&at| !Self::typed_before(class, class.index.line_begin(at)..at))
                .map(|at| Self::field(class, known, at)),
        );
        missing
    }

    fn method_missing(class: &ClassFile, method: &Method) -> Vec<MissingType> {
        let mut missing = TaggedMethod::new(class, method)
            .parameters(Self::PARAMETER, |_, part| Self::typed_before(class, part));
        missing.extend(Self::RETURN.missing(class, method));
        missing
    }

    fn typed_before(class: &ClassFile, part: Range<usize>) -> bool {
        let kept = NamePicker::without_markers(class, part);
        kept.iter()
            .take_while(|&&at| class.words.text(at) != Self::DOLLAR)
            .any(|&at| !Self::MODIFIERS.contains(&class.words.text(at)) & class.words.is_name(at))
    }

    fn is_constant(class: &ClassFile, at: usize) -> bool {
        let line = class.index.line_begin(at)..at;
        class.words.has_word(line, &[Self::CONSTANT])
    }

    fn has_type(text: &str, close: usize) -> bool {
        let rest = &text[close..];
        let header_end = rest.find(Self::HEADER_ENDS).unwrap_or(rest.len());
        rest[..header_end].contains(Self::COLON)
    }

    fn field(class: &ClassFile, known: &KnownClasses, at: usize) -> MissingType {
        let name = class.words.text(at);
        let empty = !FieldValue::has_value(class, at);
        let injected = known.injected(name).filter(|_| empty);
        let type_name = injected.unwrap_or(Self::UNKNOWN);
        let dollar = at.saturating_sub(1);
        let insertion = injected.map(|_| TypeInsertion {
            at: class.words.start(dollar),
            text: format!("{type_name} "),
        });
        MissingType::of(
            class,
            at,
            format!("field `${name}`"),
            format!("private {type_name} ${name}"),
        )
        .inserting(insertion)
    }
}
