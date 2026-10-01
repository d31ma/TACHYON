pub(super) struct ConstructorFieldPolicy;

impl ConstructorFieldPolicy {
    pub(super) fn accepts(path: &str, kind: &str, needs_all: bool) -> bool {
        Self::selection((needs_all, path.ends_with(".kt"), kind))
    }

    fn selection(selected: (bool, bool, &str)) -> bool {
        match selected {
            (true, _, _) | (false, true, "DocumentTitle") => true,
            _ => false,
        }
    }
}
