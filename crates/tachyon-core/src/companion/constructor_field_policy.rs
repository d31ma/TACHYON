pub(super) struct ConstructorFieldPolicy;

impl ConstructorFieldPolicy {
    pub(super) fn accepts(path: &str, kind: &str, needs_all: bool) -> bool {
        Self::selection((
            needs_all,
            std::path::Path::new(path)
                .extension()
                .and_then(|name| name.to_str())
                == Some("kt"),
            kind,
        ))
    }

    fn selection(selected: (bool, bool, &str)) -> bool {
        matches!(selected, (true, _, _) | (false, true, "DocumentTitle"))
    }
}
