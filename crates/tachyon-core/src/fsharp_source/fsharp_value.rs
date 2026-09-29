use super::FsharpType;

pub(crate) struct FsharpValue;

impl FsharpValue {
    const UNKNOWN: &'static str = "object";
    const LONG_SUFFIX: char = 'L';

    pub(crate) fn type_or_guess(declared: &str, initial: &str) -> String {
        let declared = declared.trim();
        let written = (!declared.is_empty()).then(|| FsharpType::csharp(declared));
        written.unwrap_or_else(|| String::from(Self::guess(initial.trim())))
    }

    fn guess(initial: &str) -> &'static str {
        Self::text(initial)
            .or_else(|| Self::flag(initial))
            .or_else(|| Self::whole(initial))
            .or_else(|| Self::long(initial))
            .or_else(|| Self::fraction(initial))
            .unwrap_or(Self::UNKNOWN)
    }

    fn text(initial: &str) -> Option<&'static str> {
        initial.starts_with('"').then_some("string")
    }

    fn flag(initial: &str) -> Option<&'static str> {
        matches!(initial, "true" | "false").then_some("bool")
    }

    fn whole(initial: &str) -> Option<&'static str> {
        Self::is_whole(initial).then_some("int")
    }

    fn long(initial: &str) -> Option<&'static str> {
        initial
            .strip_suffix(Self::LONG_SUFFIX)
            .is_some_and(Self::is_whole)
            .then_some("long")
    }

    fn fraction(initial: &str) -> Option<&'static str> {
        let digits = initial.trim_start_matches('-');
        let fraction = digits.parse::<f64>().is_ok() && digits.contains('.');
        fraction.then_some("double")
    }

    fn is_whole(initial: &str) -> bool {
        let digits = initial.trim_start_matches('-');
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpValue;

    #[test]
    fn a_written_type_wins_and_simple_literals_are_guessed() {
        assert_eq!(FsharpValue::type_or_guess("int64", "0"), "long");
        assert_eq!(FsharpValue::type_or_guess("", "6"), "int");
        assert_eq!(FsharpValue::type_or_guess("", "0L"), "long");
        assert_eq!(FsharpValue::type_or_guess("", "L"), "object");
        assert_eq!(FsharpValue::type_or_guess("", "-1.5"), "double");
        assert_eq!(FsharpValue::type_or_guess("", "\"F#\""), "string");
        assert_eq!(FsharpValue::type_or_guess("", "true"), "bool");
        assert_eq!(FsharpValue::type_or_guess("", "Store()"), "object");
    }
}
