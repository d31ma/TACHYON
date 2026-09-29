use std::fmt::Write as _;

pub(super) struct StringLiteral {
    mark: char,
    escapes_dollar: bool,
}

impl StringLiteral {
    pub(super) const SWIFT: Self = Self {
        mark: '"',
        escapes_dollar: false,
    };
    pub(super) const DART: Self = Self {
        mark: '\'',
        escapes_dollar: true,
    };
    const DOLLAR: char = '$';

    pub(super) fn of(&self, value: &str) -> String {
        let mut result = String::from(self.mark);
        for character in value.chars() {
            match character {
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                mark if mark == self.mark => {
                    result.push('\\');
                    result.push(mark);
                }
                Self::DOLLAR if self.escapes_dollar => result.push_str("\\$"),
                control if control.is_control() => {
                    let _ = write!(result, "\\u{{{:x}}}", u32::from(control));
                }
                other => result.push(other),
            }
        }
        result.push(self.mark);
        result
    }
}
