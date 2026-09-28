use super::FsharpType;

pub(crate) struct FsharpValue;

impl FsharpValue {
    const UNKNOWN: &'static str = "object";

    pub(crate) fn type_or_guess(declared: &str, initial: &str) -> String {
        let declared = declared.trim();
        let written = (!declared.is_empty()).then(|| FsharpType::csharp(declared));
        written.unwrap_or_else(|| String::from(Self::guess(initial.trim())))
    }

    fn guess(initial: &str) -> &'static str {
        let digits = initial.trim_start_matches('-');
        if initial.starts_with('"') {
            "string"
        } else if initial == "true" || initial == "false" {
            "bool"
        } else if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            "int"
        } else if digits.parse::<f64>().is_ok() && digits.contains('.') {
            "double"
        } else {
            Self::UNKNOWN
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FsharpValue;

    #[test]
    fn a_written_type_wins_and_simple_literals_are_guessed() {
        assert_eq!(FsharpValue::type_or_guess("int64", "0"), "long");
        assert_eq!(FsharpValue::type_or_guess("", "6"), "int");
        assert_eq!(FsharpValue::type_or_guess("", "-1.5"), "double");
        assert_eq!(FsharpValue::type_or_guess("", "\"F#\""), "string");
        assert_eq!(FsharpValue::type_or_guess("", "true"), "bool");
        assert_eq!(FsharpValue::type_or_guess("", "Store()"), "object");
    }
}
