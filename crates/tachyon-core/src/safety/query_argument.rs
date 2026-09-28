pub(super) struct QueryArgument;

impl QueryArgument {
    const PREFIXES: [&'static str; 9] = ["", "r", "R", "b", "B", "rb", "br", "@", "r#"];
    const QUOTES: [char; 3] = ['"', '\'', '`'];
    const PREFIX_CHARACTERS: [char; 6] = ['r', 'R', 'b', 'B', '@', '#'];
    const PHP_VARIABLE: &'static str = "$";
    const INTERPOLATION: [(&'static str, &'static str); 8] = [
        ("js", "${"),
        ("ts", "${"),
        ("mjs", "${"),
        ("rb", "#{"),
        ("php", "$"),
        ("kt", "$"),
        ("dart", "$"),
        ("swift", "\\("),
    ];

    pub(super) fn is_safe(extension: &str, source: &str, code: &str) -> bool {
        let checks = [
            Self::is_constant(source.trim()),
            Self::is_plain_literal(extension, source.trim(), code),
        ];
        checks.contains(&true)
    }

    fn is_constant(text: &str) -> bool {
        let last = text
            .rsplit(['.', ':', '>'])
            .next()
            .unwrap_or("")
            .trim_start_matches('$');
        [Self::has_upper(last), Self::constant_name(last)] == [true, true]
    }

    fn has_upper(name: &str) -> bool {
        name.bytes().any(|byte| byte.is_ascii_uppercase())
    }

    fn constant_name(name: &str) -> bool {
        name.bytes().all(Self::constant_byte)
    }

    fn constant_byte(byte: u8) -> bool {
        matches!(byte, b'A'..=b'Z' | b'0'..=b'9' | b'_')
    }

    fn is_plain_literal(extension: &str, source: &str, code: &str) -> bool {
        let prefix: String = code
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        let literal = source.trim_start_matches(Self::PREFIX_CHARACTERS);
        let checks = [
            Self::PREFIXES.contains(&prefix.as_str()),
            literal.starts_with(Self::QUOTES),
            !Self::interpolates(extension, literal),
        ];
        !checks.contains(&false)
    }

    fn interpolates(extension: &str, literal: &str) -> bool {
        Self::INTERPOLATION
            .iter()
            .any(|entry| Self::applies(extension, *entry, literal))
    }

    fn applies(extension: &str, entry: (&str, &str), literal: &str) -> bool {
        let (language, marker) = entry;
        [language == extension, Self::marks(literal, marker)] == [true, true]
    }

    fn marks(literal: &str, marker: &str) -> bool {
        [
            literal.contains(marker),
            !Self::single_quoted_php(literal, marker),
        ] == [true, true]
    }

    fn single_quoted_php(literal: &str, marker: &str) -> bool {
        literal.starts_with('\'') && marker == Self::PHP_VARIABLE
    }
}
