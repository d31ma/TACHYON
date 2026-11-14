#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuiltInError {
    name: &'static str,
    status: u16,
}

impl BuiltInError {
    pub(crate) const ROUTE: &'static str = "/.tachyon/error";
    pub(crate) const BROKEN_RULE: &'static str = "BrokenRule";

    pub(crate) const NOT_SIGNED_IN: Self = Self::new("NotSignedIn", 401);
    pub(crate) const ROLE_MISSING: Self = Self::new("RoleMissing", 403);
    pub(crate) const IDENTITY_CHECK_FAILED: Self = Self::new("IdentityCheckFailed", 500);
    pub(crate) const REQUEST_REJECTED: Self = Self::new("RequestRejected", 400);
    pub(crate) const BODY_TOO_LARGE: Self = Self::new("BodyTooLarge", 413);
    pub(crate) const CHECK_REJECTED: Self = Self::new("CheckRejected", 422);
    pub(crate) const UNEXPECTED_ERROR: Self = Self::new("UnexpectedError", 500);

    pub(crate) const ALL: [Self; 7] = [
        Self::NOT_SIGNED_IN,
        Self::ROLE_MISSING,
        Self::IDENTITY_CHECK_FAILED,
        Self::REQUEST_REJECTED,
        Self::BODY_TOO_LARGE,
        Self::CHECK_REJECTED,
        Self::UNEXPECTED_ERROR,
    ];

    const TYPE_PREFIXES: [&'static str; 2] = ["const ", "?"];
    const TYPE_MARKS: [char; 4] = ['&', '*', '?', ' '];
    const NAMESPACE_MARKS: [char; 3] = [':', '.', '\\'];

    const fn new(name: &'static str, status: u16) -> Self {
        Self { name, status }
    }

    pub(crate) const fn name(self) -> &'static str {
        self.name
    }

    pub(crate) const fn status(self) -> u16 {
        self.status
    }

    pub(crate) fn named(type_text: &str) -> Option<Self> {
        let bare = Self::bare_name(type_text);
        Self::ALL.into_iter().find(|error| error.name == bare)
    }

    pub(crate) fn is_reserved(name: &str) -> bool {
        let built_in = Self::named(name).is_some();
        [built_in, name == Self::BROKEN_RULE].contains(&true)
    }

    fn bare_name(type_text: &str) -> &str {
        let unprefixed = Self::without_prefixes(type_text.trim());
        let unmarked = unprefixed.trim_matches(Self::TYPE_MARKS);
        Self::last_segment(unmarked)
    }

    fn without_prefixes(text: &str) -> &str {
        Self::TYPE_PREFIXES.iter().fold(text, |rest, prefix| {
            rest.strip_prefix(prefix).unwrap_or(rest)
        })
    }

    fn last_segment(text: &str) -> &str {
        text.rsplit(Self::NAMESPACE_MARKS).next().unwrap_or(text)
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltInError;

    #[test]
    fn every_language_spelling_names_the_same_built_in_error() {
        let spellings = [
            "NotSignedIn",
            "*NotSignedIn",
            "&NotSignedIn",
            "const NotSignedIn&",
            "\\NotSignedIn",
            "tachyon::NotSignedIn",
            " NotSignedIn ",
        ];
        for spelling in spellings {
            assert_eq!(
                BuiltInError::named(spelling),
                Some(BuiltInError::NOT_SIGNED_IN),
                "{spelling}"
            );
        }
    }

    #[test]
    fn each_built_in_error_keeps_its_fixed_status() {
        let statuses: Vec<(&str, u16)> = BuiltInError::ALL
            .iter()
            .map(|error| (error.name(), error.status()))
            .collect();
        assert_eq!(
            statuses,
            [
                ("NotSignedIn", 401),
                ("RoleMissing", 403),
                ("IdentityCheckFailed", 500),
                ("RequestRejected", 400),
                ("BodyTooLarge", 413),
                ("CheckRejected", 422),
                ("UnexpectedError", 500),
            ]
        );
    }

    #[test]
    fn application_names_are_not_built_in() {
        assert_eq!(BuiltInError::named("OrderNotFound"), None);
        assert_eq!(BuiltInError::named("Exception"), None);
        assert!(BuiltInError::is_reserved("BrokenRule"));
        assert!(BuiltInError::is_reserved("CheckRejected"));
        assert!(!BuiltInError::is_reserved("NotSignedInYet"));
    }
}
