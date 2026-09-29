#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlatformNames {
    pub system: &'static str,
    pub arm: bool,
    pub bun: &'static str,
    pub go: &'static str,
    pub binaryen: &'static str,
    pub dart: &'static str,
    pub rust: &'static str,
}
