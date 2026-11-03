use super::super::super::cohesion::Method;
use super::super::braced_fields::BracedFields;
use super::super::class_file::ClassFile;
use super::field_value::FieldValue;
use super::known_classes::KnownClasses;
use super::missing_type::MissingType;
use super::return_value::ReturnValue;
use super::tagged_method::TaggedMethod;
use super::type_comments::TypeComments;
use super::type_insertion::TypeInsertion;
use super::type_tags::TypeTags;

pub(super) struct JavascriptTypes;

impl JavascriptTypes {
    const UNKNOWN: &'static str = "T";
    const PARAMETER: &'static str = "@param {T} NAME";
    const RETURN: &'static str = "@returns {T}";

    pub(super) fn missing(class: &ClassFile, known: &KnownClasses) -> Vec<MissingType> {
        let mut missing = TaggedMethod::each(class, |method| Self::method_missing(class, method));
        missing.extend(
            BracedFields::read(class)
                .into_iter()
                .filter_map(|at| Self::field_missing(class, known, at)),
        );
        missing
    }

    fn method_missing(class: &ClassFile, method: &Method) -> Vec<MissingType> {
        let tagged = TaggedMethod::new(class, method);
        let tags = tagged.tags(TypeTags::js_doc);
        let wanted = ReturnValue::gives_value(class, method) & !tags.returns();
        let mut missing = tagged.tagged_parameters(&tags, Self::PARAMETER);
        missing.extend(tagged.return_value(wanted, Self::RETURN));
        missing
    }

    fn field_missing(class: &ClassFile, known: &KnownClasses, at: usize) -> Option<MissingType> {
        let comments = TypeComments::above(class, at);
        let typed =
            TypeTags::js_doc(&comments.comments).typed() | FieldValue::is_literal(class, at);
        (!typed).then(|| Self::field(class, known, at, &comments))
    }

    fn field(
        class: &ClassFile,
        known: &KnownClasses,
        at: usize,
        comments: &TypeComments,
    ) -> MissingType {
        let injected = Self::injected(class, known, at);
        let example = format!("/** @type {{{}}} */", injected.unwrap_or(Self::UNKNOWN));
        let insertion = injected.map(|_| TypeInsertion {
            at: comments.top,
            text: format!("{}{example}\n", comments.indent),
        });
        MissingType::of(
            class,
            at,
            format!("field `{}`", class.words.text(at)),
            example,
        )
        .inserting(insertion)
    }

    fn injected<'k>(class: &ClassFile, known: &'k KnownClasses, at: usize) -> Option<&'k str> {
        let empty = !FieldValue::has_value(class, at);
        known.injected(class.words.text(at)).filter(|_| empty)
    }
}
