use super::EntryClass;
use std::path::PathBuf;

pub(super) struct FsharpProgram<'program> {
    pub(super) fsharp: &'program str,
    pub(super) csharp: &'program str,
    pub(super) runtime: &'program str,
    pub(super) models: &'program [(PathBuf, String)],
    pub(super) entry: EntryClass,
    pub(super) portable: &'program str,
    pub(super) framework: &'program str,
}
