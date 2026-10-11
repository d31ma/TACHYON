pub(crate) struct UnicodeName;

impl UnicodeName {
    /// Reads the Unicode letters, digits and underscores that start `text`.
    pub(crate) fn leading(text: &str) -> String {
        text.chars()
            .take_while(|character| Self::is_name_character(*character))
            .collect()
    }

    fn is_name_character(character: char) -> bool {
        [character.is_alphanumeric(), character == '_'].contains(&true)
    }
}

#[cfg(test)]
mod tests {
    use super::UnicodeName;

    #[test]
    fn leading_names_are_the_unicode_letters_digits_and_underscores_that_start_the_text() {
        let alphabet = (0..=0x7f_u8)
            .map(char::from)
            .chain(['é', 'Ü', 'ß', '٣', '漢', '·', '\u{a0}'])
            .collect::<Vec<_>>();
        let mut texts = vec![String::new()];
        for first in &alphabet {
            texts.push(first.to_string());
            texts.extend(alphabet.iter().map(|second| format!("{first}{second}")));
        }
        for text in texts {
            let expected: String = text
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect();
            assert_eq!(UnicodeName::leading(&text), expected, "{text:?}");
        }
    }
}
