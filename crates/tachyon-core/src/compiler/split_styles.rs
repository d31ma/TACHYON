#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct SplitStyles {
    pub(crate) foundation: String,
    pub(crate) component: String,
}

impl SplitStyles {
    pub(super) fn new(prefix: &str, chunks: &[(bool, &str)]) -> Self {
        let foundation = chunks.iter().filter(|(accepted, _)| *accepted);
        let component = chunks.iter().filter(|(accepted, _)| !*accepted);
        Self {
            foundation: foundation.map(|(_, text)| *text).collect(),
            component: std::iter::once(prefix)
                .chain(component.map(|(_, text)| *text))
                .collect(),
        }
    }
}
