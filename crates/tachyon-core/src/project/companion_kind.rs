use super::NativeCompanion;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompanionKind {
    Style,
    ClientModule,
    TypeScriptModule,
    Native(NativeCompanion),
}

impl CompanionKind {
    const FILE_NAMES: [(&'static str, Self); 7] = [
        ("tac.css", Self::Style),
        ("tac.js", Self::ClientModule),
        ("tac.ts", Self::TypeScriptModule),
        ("tac.rs", Self::Native(NativeCompanion::Rust)),
        ("tac.swift", Self::Native(NativeCompanion::Swift)),
        ("tac.kt", Self::Native(NativeCompanion::Kotlin)),
        ("tac.cs", Self::Native(NativeCompanion::CSharp)),
    ];

    pub(crate) fn from_file_name(name: &str) -> Option<Self> {
        Self::FILE_NAMES
            .iter()
            .find(|(file_name, _)| *file_name == name)
            .map(|(_, kind)| *kind)
    }
}
