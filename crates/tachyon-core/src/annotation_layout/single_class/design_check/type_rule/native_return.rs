use super::super::super::cohesion::Method;
use super::super::class_file::ClassFile;
use super::super::method_filter::MethodFilter;
use super::super::signature::Signature;
use super::missing_type::MissingType;
use super::return_value::ReturnValue;
use super::tagged_method::TaggedMethod;
use super::type_insertion::TypeInsertion;

pub(super) struct NativeReturn {
    pub(super) typed: fn(&str, usize) -> bool,
    pub(super) nothing: &'static str,
    pub(super) unknown: &'static str,
}

impl NativeReturn {
    pub(super) fn missing(&self, class: &ClassFile, method: &Method) -> Option<MissingType> {
        let close = Signature::of(class, method).parameters.end;
        let close = ReturnValue::closing_parenthesis(class, close)?;
        let wanted =
            !MethodFilter::is_constructor(class, method) & !(self.typed)(class.text, close);
        let nothing = !ReturnValue::gives_value(class, method);
        let example = [self.unknown, self.nothing][usize::from(nothing)].trim_start();
        let insertion = nothing.then(|| TypeInsertion {
            at: close + 1,
            text: self.nothing.to_owned(),
        });
        TaggedMethod::new(class, method)
            .return_value(wanted, example)
            .map(|missing| missing.inserting(insertion))
    }
}
