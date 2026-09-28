use super::SecretMatch;

pub(super) struct SecretAssignment;

impl SecretAssignment {
    const KIND: &'static str = "a secret value in a variable with a secret name";
    const NAMES: [&'static str; 9] = [
        "password",
        "passwd",
        "secret",
        "token",
        "apikey",
        "accesskey",
        "privatekey",
        "credential",
        "signingkey",
    ];
    const QUOTES: [char; 3] = ['"', '\'', '`'];
    const OPERATORS: [&'static str; 2] = ["=", ":"];
    const NOT_RANDOM: [&'static str; 4] = [" ", "://", "${", "{{"];
    const MIN_LENGTH: usize = 16;
    const MIN_CLASSES: usize = 2;
    const MIN_ENTROPY: f64 = 3.5;

    pub(super) fn find(text: &str, line: usize) -> Option<SecretMatch> {
        let (name, value) = Self::assignment(text)?;
        Self::is_secret(name, value).then(|| SecretMatch {
            line,
            kind: Self::KIND,
            value: String::from(value),
        })
    }

    fn is_secret(name: &str, value: &str) -> bool {
        Self::secret_name(name) && Self::looks_random(value)
    }

    fn assignment(text: &str) -> Option<(&str, &str)> {
        let opening = text.find(Self::QUOTES)?;
        let quote = text[opening..].chars().next()?;
        let rest = &text[opening + quote.len_utf8()..];
        let value = &rest[..rest.find(quote)?];
        let name = Self::assigned_name(&text[..opening])?;
        Some((name, value))
    }

    fn assigned_name(before: &str) -> Option<&str> {
        let trimmed = before
            .trim_end()
            .trim_end_matches(['>', '=', ':'])
            .trim_end();
        let operator = &before.trim_end()[trimmed.len()..];
        let assigned = Self::OPERATORS
            .iter()
            .any(|symbol| operator.contains(symbol));
        assigned.then(|| Self::last_word(trimmed))
    }

    fn last_word(text: &str) -> &str {
        let start = text
            .rfind(|character: char| !Self::name_character(character))
            .map_or(0, |at| at + 1);
        &text[start..]
    }

    fn name_character(character: char) -> bool {
        matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '$')
    }

    fn secret_name(name: &str) -> bool {
        let plain = name.replace(['_', '-', '$'], "").to_ascii_lowercase();
        Self::NAMES.iter().any(|word| plain.contains(word))
    }

    fn looks_random(value: &str) -> bool {
        let checks = [
            value.len() >= Self::MIN_LENGTH,
            Self::character_classes(value) >= Self::MIN_CLASSES,
            Self::entropy(value) >= Self::MIN_ENTROPY,
            !Self::has_words_or_templates(value),
        ];
        !checks.contains(&false)
    }

    fn has_words_or_templates(value: &str) -> bool {
        Self::NOT_RANDOM.iter().any(|part| value.contains(part))
    }

    fn character_classes(value: &str) -> usize {
        let classes = [
            Self::has(value, u8::is_ascii_lowercase),
            Self::has(value, u8::is_ascii_uppercase),
            Self::has(value, u8::is_ascii_digit),
        ];
        classes.iter().filter(|present| **present).count()
    }

    fn has(value: &str, test: fn(&u8) -> bool) -> bool {
        value.bytes().any(|byte| test(&byte))
    }

    fn entropy(value: &str) -> f64 {
        let length = f64::from(u32::try_from(value.chars().count()).unwrap_or(u32::MAX));
        Self::counts(value)
            .values()
            .map(|count| Self::share_entropy(*count, length))
            .sum()
    }

    fn share_entropy(count: u32, length: f64) -> f64 {
        let share = f64::from(count) / length;
        -share * share.log2()
    }

    fn counts(value: &str) -> std::collections::BTreeMap<char, u32> {
        let mut counts = std::collections::BTreeMap::new();
        for character in value.chars() {
            *counts.entry(character).or_insert(0_u32) += 1;
        }
        counts
    }
}
