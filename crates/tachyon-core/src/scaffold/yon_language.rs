use super::{Choice, TemplateFiles};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum YonLanguage {
    JavaScript,
    TypeScript,
    Python,
    Go,
    Java,
    Groovy,
    Kotlin,
    CSharp,
    FSharp,
    Php,
    Ruby,
    Lua,
    Rust,
    Dart,
}

impl YonLanguage {
    const OUTPUT: &'static str = "server/";
    const NAMES: [&'static str; 14] = [
        "js,javascript",
        "ts,typescript",
        "py,python",
        "go",
        "java",
        "groovy",
        "kt,kotlin",
        "cs,csharp,c#",
        "fs,fsharp,f#",
        "php",
        "rb,ruby",
        "lua",
        "rs,rust",
        "dart",
    ];
    const LABELS: [&'static str; 14] = [
        "JavaScript",
        "TypeScript",
        "Python",
        "Go",
        "Java",
        "Groovy",
        "Kotlin",
        "C#",
        "F#",
        "PHP",
        "Ruby",
        "Lua",
        "Rust",
        "Dart",
    ];

    pub(crate) fn files(self) -> Vec<(String, String)> {
        TemplateFiles::under(&format!("yon/{}", self.flag_name()), Self::OUTPUT)
    }
}

impl Choice for YonLanguage {
    const ALL: &'static [Self] = &[
        Self::JavaScript,
        Self::TypeScript,
        Self::Python,
        Self::Go,
        Self::Java,
        Self::Groovy,
        Self::Kotlin,
        Self::CSharp,
        Self::FSharp,
        Self::Php,
        Self::Ruby,
        Self::Lua,
        Self::Rust,
        Self::Dart,
    ];
    const QUESTION: &'static str =
        "Which languages do you want for the server (Yon)? You can choose more than one.";

    fn names(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn label(self) -> &'static str {
        Self::LABELS[self as usize]
    }
}
