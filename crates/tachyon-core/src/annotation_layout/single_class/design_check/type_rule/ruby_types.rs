use super::super::super::cohesion::Method;
use super::super::class_file::ClassFile;
use super::super::line_fields::LineFields;
use super::super::method_filter::MethodFilter;
use super::known_classes::KnownClasses;
use super::missing_type::MissingType;
use super::tagged_method::TaggedMethod;
use super::type_comments::TypeComments;
use super::type_insertion::TypeInsertion;
use super::type_tags::TypeTags;

pub(super) struct RubyTypes;

impl RubyTypes {
    const ACCESSORS: [&'static str; 3] = ["attr_reader", "attr_accessor", "attr_writer"];
    const UNKNOWN: &'static str = "T";
    const PARAMETER: &'static str = "# @param NAME [T]";
    const RETURN: &'static str = "# @return [T]";

    pub(super) fn missing(class: &ClassFile, known: &KnownClasses) -> Vec<MissingType> {
        let mut missing = TaggedMethod::each(class, |method| Self::method_missing(class, method));
        missing.extend(
            LineFields::read(class)
                .into_iter()
                .filter(|&at| Self::is_accessor(class, at))
                .filter_map(|at| Self::field_missing(class, known, at)),
        );
        missing
    }

    fn method_missing(class: &ClassFile, method: &Method) -> Vec<MissingType> {
        let tagged = TaggedMethod::new(class, method);
        let tags = tagged.tags(TypeTags::yard);
        let wanted = !MethodFilter::is_constructor(class, method) & !tags.returns();
        let mut missing = tagged.tagged_parameters(&tags, Self::PARAMETER);
        missing.extend(tagged.return_value(wanted, Self::RETURN));
        missing
    }

    fn is_accessor(class: &ClassFile, at: usize) -> bool {
        let line = class.index.line_begin(at);
        Self::ACCESSORS.contains(&class.words.text(line))
    }

    fn field_missing(class: &ClassFile, known: &KnownClasses, at: usize) -> Option<MissingType> {
        let comments = TypeComments::above(class, at);
        let typed = TypeTags::yard(&comments.comments).returns();
        (!typed).then(|| Self::field(class, known, at, &comments))
    }

    fn field(
        class: &ClassFile,
        known: &KnownClasses,
        at: usize,
        comments: &TypeComments,
    ) -> MissingType {
        let name = class.words.text(at);
        let injected = known.injected(name);
        let example = format!("# @return [{}]", injected.unwrap_or(Self::UNKNOWN));
        let insertion = injected.map(|_| TypeInsertion {
            at: comments.line,
            text: format!("{}{example}\n", comments.indent),
        });
        MissingType::of(class, at, format!("field `{name}`"), example).inserting(insertion)
    }
}
