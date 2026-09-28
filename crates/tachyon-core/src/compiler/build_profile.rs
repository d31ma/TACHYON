#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BuildProfile {
    #[default]
    Release,
    Development,
}
