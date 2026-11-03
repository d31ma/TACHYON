#![allow(clippy::needless_bitwise_bool)]

use super::name_parts::NameParts;

pub(super) struct NameForms;

impl NameForms {
    pub(super) fn of(name: &str) -> [String; 3] {
        [
            name.to_owned(),
            Self::lower_camel(name),
            NameParts::lower(name),
        ]
    }

    fn lower_camel(name: &str) -> String {
        let upper = name.chars().take_while(char::is_ascii_uppercase).count();
        let lowered = Self::lowered_count(upper, name.len());
        format!(
            "{}{}",
            name[..lowered].to_ascii_lowercase(),
            &name[lowered..]
        )
    }

    fn lowered_count(upper: usize, length: usize) -> usize {
        let acronym = (upper > 1) & (upper < length);
        upper - usize::from(acronym)
    }
}
