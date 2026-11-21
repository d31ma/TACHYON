pub(super) struct RootErrorType;

impl RootErrorType {
    pub(super) const JAVA: [&'static str; 4] = [
        "Exception",
        "Throwable",
        "java.lang.Exception",
        "java.lang.Throwable",
    ];
    pub(super) const KOTLIN: [&'static str; 6] = [
        "Exception",
        "Throwable",
        "kotlin.Exception",
        "kotlin.Throwable",
        "java.lang.Exception",
        "java.lang.Throwable",
    ];
    pub(super) const CSHARP: [&'static str; 3] = ["Exception", "System.Exception", "exn"];
    pub(super) const JAVASCRIPT: [&'static str; 1] = ["Error"];

    pub(super) fn condition(roots: &[&str], kind: &str, condition: &'static str) -> &'static str {
        let root = roots.contains(&kind);
        Self::condition_when(root, condition)
    }

    fn condition_when(root: bool, condition: &'static str) -> &'static str {
        if !root {
            return "";
        }
        condition
    }
}
