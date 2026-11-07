#![allow(clippy::needless_bitwise_bool)]

use super::super::cohesion::Method;
use super::annotation_marks::AnnotationMarks;
use super::braced_fields::BracedFields;
use super::class_file::ClassFile;
use super::class_member::ClassMember;
use super::field_privacy::FieldPrivacy;
use super::line_fields::LineFields;
use super::member_uses::MemberUses;
use super::method_filter::MethodFilter;
use super::signature::Signature;
use crate::annotation_rules::AnnotationRules;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct MemberReader;

impl MemberReader {
    const FIXED_NAMES: [&'static str; 12] = [
        "hotState",
        "hotDispose",
        "restoreHotState",
        "toString",
        "toJSON",
        "toJson",
        "hashCode",
        "equals",
        "String",
        "Error",
        "to_s",
        "behaviors",
    ];
    const CALLED_BY_CLASS_MARK: [(&'static str, &'static str); 1] =
        [("IdentityProvider", "identify")];
    const GO: &'static str = "go";
    const RUBY: &'static str = "rb";
    const EQUALS: &'static str = "=";
    const LONGER_EQUALS: [&'static str; 3] = ["==", "=>", "=~"];
    const COMPARISONS: [&'static str; 4] = ["==", "!=", "<=", ">="];
    const RUBY_ACCESSORS: [&'static str; 3] = ["attr_reader", "attr_accessor", "attr_writer"];

    pub(super) fn public_members(class: &ClassFile) -> Vec<ClassMember> {
        let framework_lines = Self::framework_lines(class);
        let mut members = Self::methods(class);
        members.extend(Self::fields(class));
        members.sort_by_key(|member| member.at);
        members
            .into_iter()
            .filter(|member| Self::is_chosen(class, member, &framework_lines))
            .collect()
    }

    fn framework_lines(class: &ClassFile) -> BTreeSet<usize> {
        AnnotationRules::framework_members(Path::new(class.path), class.text)
            .into_iter()
            .map(|offset| class.line(offset))
            .collect()
    }

    fn methods(class: &ClassFile) -> Vec<ClassMember> {
        class
            .methods
            .iter()
            .filter(|method| Self::is_public_method(class, method))
            .map(|method| ClassMember {
                name: method.name.clone(),
                at: Signature::of(class, method).name_at,
                field: false,
            })
            .collect()
    }

    fn is_public_method(class: &ClassFile, method: &Method) -> bool {
        let chosen = MethodFilter::names_are_chosen(class, method);
        let constructor = MethodFilter::is_constructor(class, method);
        let free_function = (class.extension == Self::GO) & method.receiver.is_empty();
        method.public & chosen & !constructor & !free_function
    }

    fn fields(class: &ClassFile) -> Vec<ClassMember> {
        let mut found = BracedFields::read(class);
        found.extend(LineFields::read(class));
        found
            .into_iter()
            .filter(|&at| Self::is_public_field(class, at))
            .map(|at| ClassMember {
                name: class.words.text(at).to_owned(),
                at,
                field: true,
            })
            .collect()
    }

    fn is_public_field(class: &ClassFile, at: usize) -> bool {
        let ruby = class.extension == Self::RUBY;
        let line = class.index.line_begin(at);
        let accessor = class.words.has_word(line..at, &Self::RUBY_ACCESSORS);
        let checked = [Self::has_starting_value(class, at), accessor][usize::from(ruby)];
        let class_level = Self::at_class_level(class, at);
        checked & class_level & !FieldPrivacy::is_private(class, at)
    }

    fn at_class_level(class: &ClassFile, at: usize) -> bool {
        let indent = class
            .methods
            .first()
            .map(|method| Self::line_indent(class, method.head));
        indent.is_none_or(|column| column == Self::line_indent(class, at))
    }

    fn line_indent(class: &ClassFile, at: usize) -> usize {
        class.words.column(class.index.line_begin(at))
    }

    fn has_starting_value(class: &ClassFile, at: usize) -> bool {
        let rest = at + 1..class.index.line_end(at);
        rest.clone().any(|index| {
            let equals = class.words.text(index) == Self::EQUALS;
            let longer = Self::LONGER_EQUALS.contains(&class.words.pair(index));
            let compared = Self::COMPARISONS.contains(&class.words.pair(index.wrapping_sub(1)));
            equals & !longer & !compared
        })
    }

    fn is_chosen(
        class: &ClassFile,
        member: &ClassMember,
        framework_lines: &BTreeSet<usize>,
    ) -> bool {
        let start = class.words.start(member.at);
        let line = class.line(start);
        let marked = AnnotationMarks::precede(class.text, start, class.extension);
        let annotated = framework_lines.contains(&line) | marked;
        let language_name = Self::FIXED_NAMES.contains(&member.name.as_str());
        !(annotated | language_name | Self::called_by_class_mark(class, member))
    }

    fn called_by_class_mark(class: &ClassFile, member: &ClassMember) -> bool {
        let name = MemberUses::plain(&member.name);
        Self::CALLED_BY_CLASS_MARK
            .iter()
            .any(|(mark, called)| (name == *called) & class.text.contains(mark))
    }
}
