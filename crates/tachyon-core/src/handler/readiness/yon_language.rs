use std::ffi::OsStr;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum YonLanguage {
    JavaScript,
    TypeScript,
    Python,
    Java,
    Groovy,
    Php,
    Kotlin,
    CSharp,
    FSharp,
    Rust,
    Dart,
    Go,
    Ruby,
    Lua,
    Cpp,
}

impl YonLanguage {
    pub(crate) fn from_extension(extension: &str) -> Option<Self> {
        Self::from_lowercase_extension(&extension.to_ascii_lowercase())
    }

    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(OsStr::to_str)
            .and_then(Self::from_extension)
    }

    pub(crate) const fn family(self) -> &'static str {
        self.family_name()
    }

    fn from_lowercase_extension(extension: &str) -> Option<Self> {
        match extension {
            "js" => Some(Self::JavaScript),
            "ts" => Some(Self::TypeScript),
            "py" => Some(Self::Python),
            "java" => Some(Self::Java),
            "groovy" => Some(Self::Groovy),
            "php" => Some(Self::Php),
            "kt" => Some(Self::Kotlin),
            "cs" => Some(Self::CSharp),
            "fs" => Some(Self::FSharp),
            "rs" => Some(Self::Rust),
            "dart" => Some(Self::Dart),
            "go" => Some(Self::Go),
            "rb" => Some(Self::Ruby),
            "lua" => Some(Self::Lua),
            "hpp" => Some(Self::Cpp),
            _ => None,
        }
    }

    const fn family_name(self) -> &'static str {
        match self {
            Self::JavaScript | Self::TypeScript => "javascript",
            Self::Python => "python",
            Self::Java => "java",
            Self::Groovy => "groovy",
            Self::Php => "php",
            Self::Kotlin => "kotlin",
            Self::CSharp => "csharp",
            Self::FSharp => "fsharp",
            Self::Rust => "rust",
            Self::Dart => "dart",
            Self::Go => "go",
            Self::Ruby => "ruby",
            Self::Lua => "lua",
            Self::Cpp => "cpp",
        }
    }
}
