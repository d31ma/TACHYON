#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SourceAction {
    Css,
    Island { boundaries: Vec<String> },
    Reload,
}
