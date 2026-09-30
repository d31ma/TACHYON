use super::{Choice, TemplateFiles};

type LanguageRow = (&'static str, &'static str, &'static str, &'static str);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TacLanguage {
    JavaScript,
    TypeScript,
    Dart,
    Rust,
    CSharp,
    FSharp,
    Go,
    Ruby,
    Lua,
    Swift,
    Kotlin,
    Java,
}

impl TacLanguage {
    const OUTPUT: &'static str = "client/";
    const ROWS: [LanguageRow; 11] = [
        ("js,javascript", "JavaScript", "shell", ""),
        ("ts,typescript", "TypeScript", "typeScriptGreeting", ""),
        ("dart", "Dart", "dartGreeting", ""),
        ("rs,rust", "Rust", "rustGreeting", ""),
        ("cs,csharp,c#", "C#", "cSharpGreeting", ""),
        ("fs,fsharp,f#", "F#", "fSharpGreeting", ""),
        ("go", "Go", "goGreeting", ""),
        ("rb,ruby", "Ruby", "rubyGreeting", ""),
        ("lua", "Lua", "luaGreeting", ""),
        (
            "swift",
            "Swift",
            "swiftGreeting",
            "Swift services run in the macOS and iOS apps. The web and the other targets use the JavaScript copy of the service.",
        ),
        (
            "kt,kotlin",
            "Kotlin",
            "kotlinGreeting",
            "Kotlin services run in the Android app. The web and the other targets use the JavaScript copy of the service.",
        ),
        (
            "java",
            "Java",
            "javaGreeting",
            "Java services reach every target as WebAssembly through TeaVM 0.15.0. Tachyon can install TeaVM into its cache, or you point TAC_TEAVM_HOME at the TeaVM jars.",
        ),
    ];

    #[must_use]
    pub fn note(self) -> &'static str {
        self.row().3
    }

    pub(crate) fn slug(self) -> &'static str {
        self.names().split(',').nth(1).unwrap_or(self.flag_name())
    }

    pub(crate) fn instance(self) -> &'static str {
        self.row().2
    }

    pub(crate) fn files(self) -> Vec<(String, String)> {
        TemplateFiles::under(&format!("tac/{}", self.flag_name()), Self::OUTPUT)
    }

    fn row(self) -> LanguageRow {
        Self::ROWS[self as usize]
    }
}

impl Choice for TacLanguage {
    const ALL: &'static [Self] = &[
        Self::JavaScript,
        Self::TypeScript,
        Self::Dart,
        Self::Rust,
        Self::CSharp,
        Self::FSharp,
        Self::Go,
        Self::Ruby,
        Self::Lua,
        Self::Swift,
        Self::Kotlin,
        Self::Java,
    ];
    const QUESTION: &'static str =
        "Which languages do you want for the pages (Tac)? You can choose more than one.";

    fn names(self) -> &'static str {
        self.row().0
    }

    fn label(self) -> &'static str {
        self.row().1
    }
}
