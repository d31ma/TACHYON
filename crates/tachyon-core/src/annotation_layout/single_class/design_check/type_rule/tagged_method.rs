use super::super::super::cohesion::Method;
use super::super::class_file::ClassFile;
use super::super::signature::Signature;
use super::method_parameters::MethodParameters;
use super::missing_type::MissingType;
use super::type_comments::TypeComments;
use super::type_tags::TypeTags;
use std::ops::Range;

pub(super) struct TaggedMethod<'c, 'a> {
    class: &'c ClassFile<'a>,
    method: &'c Method,
}

impl<'c, 'a> TaggedMethod<'c, 'a> {
    const NAME: &'static str = "NAME";

    pub(super) fn each(
        class: &'c ClassFile<'a>,
        missing: impl Fn(&'c Method) -> Vec<MissingType>,
    ) -> Vec<MissingType> {
        class.methods.iter().flat_map(missing).collect()
    }

    pub(super) const fn new(class: &'c ClassFile<'a>, method: &'c Method) -> Self {
        Self { class, method }
    }

    pub(super) fn tags(&self, read: fn(&str) -> TypeTags) -> TypeTags {
        let name_at = Signature::of(self.class, self.method).name_at;
        read(&TypeComments::above(self.class, name_at).comments)
    }

    pub(super) fn parameters(
        &self,
        form: &str,
        typed: impl Fn(usize, Range<usize>) -> bool,
    ) -> Vec<MissingType> {
        MethodParameters::read(self.class, self.method)
            .into_iter()
            .filter(|(at, part)| !typed(*at, part.clone()))
            .map(|(at, _)| self.parameter(at, form))
            .collect()
    }

    pub(super) fn tagged_parameters(&self, tags: &TypeTags, form: &str) -> Vec<MissingType> {
        self.parameters(form, |at, _| tags.names(self.class.words.text(at)))
    }

    pub(super) fn return_value(&self, wanted: bool, example: &str) -> Option<MissingType> {
        wanted.then(|| {
            MissingType::of(
                self.class,
                Signature::of(self.class, self.method).name_at,
                format!("return value of `{}`", self.method.name),
                example.to_owned(),
            )
        })
    }

    fn parameter(&self, at: usize, form: &str) -> MissingType {
        let name = self.class.words.text(at);
        MissingType::of(
            self.class,
            at,
            format!("parameter `{name}` of `{}`", self.method.name),
            form.replace(Self::NAME, name),
        )
    }
}
