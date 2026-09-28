pub(in super::super) struct ImportWords;

impl ImportWords {
    const JAVASCRIPT: &'static [&'static str] = &["import"];
    const PYTHON: &'static [&'static str] = &["import", "from"];
    const RUBY: &'static [&'static str] = &["require", "require_relative"];
    const PHP: &'static [&'static str] =
        &["use", "require", "require_once", "include", "include_once"];
    const JVM: &'static [&'static str] = &["import", "package"];
    const CSHARP: &'static [&'static str] = &["using"];
    const RUST: &'static [&'static str] = &["use", "extern"];
    const DART: &'static [&'static str] = &["import", "export", "part", "library"];
    const NONE: &'static [&'static str] = &[];

    pub(in super::super) fn of(extension: &str) -> &'static [&'static str] {
        Self::words_for(extension)
    }

    fn words_for(extension: &str) -> &'static [&'static str] {
        match extension {
            "js" | "ts" | "swift" => Self::JAVASCRIPT,
            "py" => Self::PYTHON,
            "rb" => Self::RUBY,
            "php" => Self::PHP,
            "java" | "kt" | "go" => Self::JVM,
            "cs" => Self::CSHARP,
            "rs" => Self::RUST,
            "dart" => Self::DART,
            _ => Self::NONE,
        }
    }
}
