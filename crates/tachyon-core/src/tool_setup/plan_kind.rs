#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanKind {
    Automatic,
    NeedsAdmin,
    Manual,
}
