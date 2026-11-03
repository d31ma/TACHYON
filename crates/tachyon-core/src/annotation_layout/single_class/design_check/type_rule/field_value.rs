use super::super::class_file::ClassFile;

pub(super) struct FieldValue;

impl FieldValue {
    const EQUALS: &'static str = "=";
    const MARKS: [&'static str; 2] = ["!", "?"];
    const BOOLEANS: [&'static str; 4] = ["true", "false", "True", "False"];
    const ENDS: [&'static str; 2] = [";", ","];

    pub(super) fn after_name(class: &ClassFile, at: usize) -> usize {
        let next = at + 1;
        next + usize::from(Self::MARKS.contains(&class.words.text(next)))
    }

    pub(super) fn has_value(class: &ClassFile, at: usize) -> bool {
        class.words.text(Self::after_name(class, at)) == Self::EQUALS
    }

    pub(super) fn is_literal(class: &ClassFile, at: usize) -> bool {
        let assigned = class.words.text(at + 1) == Self::EQUALS;
        let literal = Self::is_plain_literal(class, at + 2);
        let alone = Self::ends_at(class, at + 3);
        assigned & literal & alone
    }

    fn is_plain_literal(class: &ClassFile, at: usize) -> bool {
        let text = class.words.text(at);
        let number = text.starts_with(|character: char| character.is_ascii_digit());
        class.words.is_literal(at) | number | Self::BOOLEANS.contains(&text)
    }

    fn ends_at(class: &ClassFile, at: usize) -> bool {
        class.words.line_start(at) | Self::ENDS.contains(&class.words.text(at))
    }
}
