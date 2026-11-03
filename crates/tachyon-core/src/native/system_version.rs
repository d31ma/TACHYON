use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct SystemVersion {
    major: u32,
    minor: u32,
}

impl SystemVersion {
    const MOST_DIGITS: usize = 4;

    pub(super) const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub(super) fn parse(text: &str) -> Option<Self> {
        let (major, minor) = text.split_once('.').unwrap_or((text, "0"));
        Some(Self::new(Self::number(major)?, Self::number(minor)?))
    }

    pub(super) const fn major(self) -> u32 {
        self.major
    }

    pub(super) const fn minor(self) -> u32 {
        self.minor
    }

    fn number(part: &str) -> Option<u32> {
        let digits = !part.is_empty()
            && part.len() <= Self::MOST_DIGITS
            && part.bytes().all(|byte| byte.is_ascii_digit());
        digits.then(|| part.parse().ok()).flatten()
    }
}

impl fmt::Display for SystemVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::SystemVersion;

    #[test]
    fn a_version_has_a_major_and_an_optional_minor_number() {
        assert_eq!(
            SystemVersion::parse("18.2"),
            Some(SystemVersion::new(18, 2))
        );
        assert_eq!(SystemVersion::parse("15"), Some(SystemVersion::new(15, 0)));
        assert_eq!(SystemVersion::new(2, 46).to_string(), "2.46");
        for text in ["", "18.", ".2", "18.2.1", "v18", "18.x", "12345", " 18"] {
            assert_eq!(SystemVersion::parse(text), None, "{text:?}");
        }
    }

    #[test]
    fn versions_order_by_major_then_minor() {
        assert!(SystemVersion::new(2, 46) > SystemVersion::new(2, 40));
        assert!(SystemVersion::new(15, 2) > SystemVersion::new(13, 9));
        assert!(SystemVersion::new(18, 2) >= SystemVersion::new(18, 2));
    }
}
