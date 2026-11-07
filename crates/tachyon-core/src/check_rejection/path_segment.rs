#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum PathSegment {
    Name(String),
    Item(u32),
}
