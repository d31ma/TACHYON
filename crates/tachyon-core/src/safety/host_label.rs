pub(crate) struct HostLabel;

impl HostLabel {
    const SEPARATOR: char = '.';
    const HYPHEN: char = '-';
    const MAX_LENGTH: usize = 63;

    pub(crate) fn is_name(name: &str) -> bool {
        name.split(Self::SEPARATOR).all(Self::is_label)
    }

    fn is_label(label: &str) -> bool {
        let edges = [
            label.starts_with(Self::HYPHEN),
            label.ends_with(Self::HYPHEN),
        ];
        let length_valid = (1..=Self::MAX_LENGTH).contains(&label.len());
        let characters_valid = label.chars().all(Self::is_label_character);
        [length_valid, characters_valid, edges == [false, false]] == [true, true, true]
    }

    fn is_label_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == Self::HYPHEN
    }
}
