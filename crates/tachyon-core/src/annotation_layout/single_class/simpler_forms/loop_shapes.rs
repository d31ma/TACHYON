use super::code_language::CodeLanguage;

pub(super) struct LoopShapes {
    pub(super) headers: Vec<String>,
    pub(super) items: &'static [&'static str],
    pub(super) declarations: &'static [&'static str],
    pub(super) suffixes: &'static [&'static str],
    pub(super) output: &'static str,
}

impl LoopShapes {
    const ANY_DECLARATION: &'static str = "*";
    const UPDATES: [&'static str; 3] = ["@I ++", "++ @I", "@I += 1"];
    const PHP_UPDATES: [&'static str; 3] = ["$ @I ++", "++ $ @I", "$ @I += 1"];
    const INDEXED: &'static [&'static str] = &["= %XS [ @I ]"];
    const GETTER_SUFFIX: &'static str = ".get";

    pub(super) fn of(language: CodeLanguage) -> Self {
        Self::for_language(language)
    }

    fn for_language(language: CodeLanguage) -> Self {
        match language {
            CodeLanguage::JavaScript | CodeLanguage::TypeScript => Self {
                suffixes: &[".length"],
                ..Self::counted(
                    &["let"],
                    &["%BOUND"],
                    &["const", "let"],
                    "for ({D} {X} of {XS}) {",
                )
            },
            CodeLanguage::Java => Self {
                items: &["= %XS [ @I ]", "= %GETTER ( @I )"],
                suffixes: &[".length", ".size"],
                ..Self::counted(
                    &["int"],
                    &["%BOUND", "%BOUND ( )"],
                    &[Self::ANY_DECLARATION],
                    "for ({D} {X} : {XS}) {",
                )
            },
            CodeLanguage::CSharp => Self {
                suffixes: &[".Length", ".Count"],
                ..Self::counted(
                    &["int", "var"],
                    &["%BOUND"],
                    &[Self::ANY_DECLARATION],
                    "foreach ({D} {X} in {XS}) {",
                )
            },
            CodeLanguage::Dart => Self {
                suffixes: &[".length"],
                ..Self::counted(
                    &["var", "int"],
                    &["%BOUND"],
                    &[Self::ANY_DECLARATION],
                    "for ({D} {X} in {XS}) {",
                )
            },
            CodeLanguage::Php => Self::php(),
            CodeLanguage::Go => Self::go(),
            CodeLanguage::Kotlin => Self {
                suffixes: &[".size", ".indices"],
                ..Self::ranged(
                    &["for ( @I in 0 until %BOUND ) {", "for ( @I in %BOUND ) {"],
                    &["val"],
                    "for ({X} in {XS}) {",
                )
            },
            CodeLanguage::Swift => Self {
                suffixes: &[".count", ".indices"],
                ..Self::ranged(
                    &["for @I in 0 ..< %BOUND {", "for @I in %BOUND {"],
                    &["let"],
                    "for {X} in {XS} {",
                )
            },
            CodeLanguage::Python => Self::ranged(
                &["for @I in range ( len ( %XS ) ) :"],
                &[""],
                "for {X} in {XS}:",
            ),
            CodeLanguage::Ruby => Self {
                suffixes: &[".length", ".size", ".each_index"],
                ..Self::ranged(
                    &["( 0 ... %BOUND ) . each do | @I |", "%BOUND do | @I |"],
                    &[""],
                    "{XS}.each do |{X}|",
                )
            },
            CodeLanguage::Rust => Self::ranged(&[], &[], ""),
        }
    }

    pub(super) fn allows(&self, declaration: &str) -> bool {
        let any = self.declarations.contains(&Self::ANY_DECLARATION) & !declaration.is_empty();
        any | self.declarations.contains(&declaration)
    }

    pub(super) fn bounds(&self, bound: &str, items: &str) -> bool {
        self.suffixes
            .iter()
            .any(|suffix| bound.strip_suffix(suffix) == Some(items))
    }

    pub(super) fn items_of(indexed: &str, getter: &str) -> String {
        let listed = getter.strip_suffix(Self::GETTER_SUFFIX).unwrap_or("");
        format!("{indexed}{listed}")
    }

    fn counted(
        starts: &[&str],
        bounds: &[&str],
        declarations: &'static [&'static str],
        output: &'static str,
    ) -> Self {
        let headers = starts
            .iter()
            .flat_map(|start| bounds.iter().map(move |bound| (*start, *bound)))
            .flat_map(|(start, bound)| {
                Self::UPDATES.iter().map(move |update| {
                    format!("for ( {start} @I = 0 ; @I < {bound} ; {update} ) {{")
                })
            })
            .collect();
        Self {
            headers,
            items: Self::INDEXED,
            declarations,
            suffixes: &[],
            output,
        }
    }

    fn ranged(
        headers: &[&str],
        declarations: &'static [&'static str],
        output: &'static str,
    ) -> Self {
        Self {
            headers: headers.iter().map(|header| (*header).to_owned()).collect(),
            items: Self::INDEXED,
            declarations,
            suffixes: &[],
            output,
        }
    }

    fn php() -> Self {
        Self {
            headers: Self::PHP_UPDATES
                .iter()
                .map(|update| format!("for ( $ @I = 0 ; $ @I < count ( $ %XS ) ; {update} ) {{"))
                .collect(),
            items: &["= $ %XS [ $ @I ]"],
            declarations: &["$"],
            suffixes: &[],
            output: "foreach (${XS} as ${X}) {",
        }
    }

    fn go() -> Self {
        Self {
            headers: Self::UPDATES
                .iter()
                .map(|update| format!("for @I := 0 ; @I < len ( %XS ) ; {update} {{"))
                .collect(),
            items: &[":= %XS [ @I ]"],
            declarations: &[""],
            suffixes: &[],
            output: "for _, {X} := range {XS} {",
        }
    }
}
