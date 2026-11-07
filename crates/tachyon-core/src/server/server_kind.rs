#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ServerKind {
    #[default]
    Development,
    Production,
}
