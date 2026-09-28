use super::super::super::cohesion::Method;
use super::super::class_file::ClassFile;
use super::super::name_picker::NamePicker;
use super::super::signature::Signature;
use std::ops::Range;

pub(super) struct MethodParameters;

impl MethodParameters {
    const RECEIVERS: [&'static str; 3] = ["self", "cls", "this"];

    pub(super) fn read(class: &ClassFile, method: &Method) -> Vec<(usize, Range<usize>)> {
        let inside = Signature::of(class, method).parameters;
        NamePicker::split_at_commas(class, inside)
            .into_iter()
            .filter_map(|part| Self::named(class, part))
            .collect()
    }

    pub(super) fn has_base_word(class: &ClassFile, part: Range<usize>, words: &[&str]) -> bool {
        let kept = NamePicker::without_markers(class, part);
        let base = kept.first().map_or(0, |&at| class.words.depth(at));
        kept.iter()
            .any(|&at| (class.words.depth(at) == base) & words.contains(&class.words.text(at)))
    }

    fn named(class: &ClassFile, part: Range<usize>) -> Option<(usize, Range<usize>)> {
        let at = NamePicker::parameter_name(class, part.clone())?;
        let receiver = Self::RECEIVERS.contains(&class.words.text(at));
        (!receiver).then_some((at, part))
    }
}
