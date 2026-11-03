#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Name,
    Literal,
    Symbol,
}
