use super::super::piece::Piece;

pub(super) struct TypeName;

impl TypeName {
    const MODIFIERS: [&'static str; 23] = [
        "export",
        "default",
        "declare",
        "public",
        "private",
        "protected",
        "internal",
        "fileprivate",
        "open",
        "abstract",
        "sealed",
        "final",
        "readonly",
        "static",
        "partial",
        "data",
        "value",
        "inline",
        "pub",
        "indirect",
        "base",
        "unsafe",
        "fun",
    ];
    const KINDS: [&'static str; 12] = [
        "class",
        "struct",
        "record",
        "enum",
        "interface",
        "type",
        "typealias",
        "typedef",
        "object",
        "trait",
        "protocol",
        "mixin",
    ];
    const SECOND_KINDS: [&'static str; 2] = ["class", "struct"];
    const ATTRIBUTE_STARTS: [&'static str; 2] = ["[", "#["];

    pub(super) fn read(item: Piece<'_>) -> Option<&str> {
        let head = Self::after_attributes(item.after_annotations());
        let head = Self::after_modifiers(head);
        let rest = Self::after_kind(head)?;
        Self::identifier(rest.after_words(&Self::SECOND_KINDS))
    }

    fn after_attributes(mut head: Piece<'_>) -> Piece<'_> {
        while Self::is_attribute(head) {
            head = Self::after_attribute(head);
        }
        head.after_annotations()
    }

    fn is_attribute(head: Piece<'_>) -> bool {
        Self::ATTRIBUTE_STARTS
            .iter()
            .any(|start| head.starts_with(start))
    }

    fn after_attribute(head: Piece<'_>) -> Piece<'_> {
        let open = usize::from(head.starts_with("#"));
        let group = head.from(open);
        group.after_group().unwrap_or(head.from(head.text.len()))
    }

    fn after_modifiers(mut head: Piece<'_>) -> Piece<'_> {
        while Self::MODIFIERS.contains(&head.first_word()) {
            head = head.from(head.first_word().len()).trim().after_arguments();
        }
        head
    }

    fn after_kind(head: Piece<'_>) -> Option<Piece<'_>> {
        let kind = head.first_word();
        Self::KINDS
            .contains(&kind)
            .then(|| head.from(kind.len()).trim())
    }

    fn identifier(rest: Piece<'_>) -> Option<&str> {
        let name = rest.first_word();
        let starts_as_name = name
            .bytes()
            .next()
            .is_some_and(|byte| !byte.is_ascii_digit());
        starts_as_name.then_some(name)
    }
}
