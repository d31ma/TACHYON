#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum ErrorIdiom {
    None,
    GoErrorCheck,
    SwiftGuard,
    KotlinElvis,
}
