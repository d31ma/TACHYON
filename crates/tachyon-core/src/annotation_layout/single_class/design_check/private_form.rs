pub(super) struct PrivateForm;

impl PrivateForm {
    const KEYWORD: &'static str = "write `private` before it, such as `private total`";
    const UNDERSCORE: &'static str = "start its name with `_`, such as `_total`";
    const FORMS: [(&'static str, &'static str); 12] = [
        (
            "js",
            "start its name with `_`, such as `_total` or `_pick(value)`",
        ),
        ("ts", Self::KEYWORD),
        ("java", Self::KEYWORD),
        ("kt", Self::KEYWORD),
        ("cs", Self::KEYWORD),
        ("swift", Self::KEYWORD),
        ("php", Self::KEYWORD),
        ("dart", Self::UNDERSCORE),
        ("py", Self::UNDERSCORE),
        (
            "rb",
            "write `private` before its `def` or `attr_reader`, such as `private def total`",
        ),
        (
            "go",
            "start its name with a lowercase letter, such as `total`",
        ),
        ("rs", "remove `pub`, such as `fn total(&self)`"),
    ];

    pub(super) fn of(extension: &str) -> &'static str {
        Self::FORMS
            .iter()
            .find(|(language, _)| *language == extension)
            .map_or(Self::KEYWORD, |(_, form)| form)
    }
}
