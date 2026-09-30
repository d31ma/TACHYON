pub(super) struct ConstructorParameter<'a> {
    pub(super) text: &'a str,
    pub(super) name: &'a str,
    pub(super) type_name: &'a str,
}

impl<'a> ConstructorParameter<'a> {
    const TYPE_SEPARATOR: char = ':';

    pub(super) fn read(text: &'a str) -> Self {
        let (name, type_name) = text.split_once(Self::TYPE_SEPARATOR).unwrap_or((text, ""));
        Self {
            text,
            name: name.trim(),
            type_name: type_name.trim(),
        }
    }
}
