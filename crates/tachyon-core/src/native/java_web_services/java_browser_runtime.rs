pub(super) struct JavaBrowserRuntime;

impl JavaBrowserRuntime {
    const BUILTINS: &'static str = include_str!("BehaviorBuiltins.java");
    const TEA_VM_SWAPS: [(&'static str, &'static str); 8] = [
        (
            "static class Failure extends RuntimeException {\n        final String code;\n",
            "static class Failure extends TacFailure {\n",
        ),
        (
            "Failure(String code, String path, String message) { super(message); this.code = code; this.path = path; }",
            "Failure(String code, String path, String message) { super(code, message); this.path = path; }",
        ),
        ("java.util.concurrent.Semaphore", "TacPermits"),
        ("java.util.Map.copyOf(", "TacMaps.copyOf("),
        ("java.util.StringJoiner", "TacJoiner"),
        ("Thread.sleep(", "TacPause.sleep("),
        ("synchronized (this) ", ""),
        ("static synchronized ", "static "),
    ];
    const REGEX_FREE_SWAPS: [(&'static str, &'static str); 10] = [
        (
            r#"((String)profile).matches("[A-Za-z_$][A-Za-z0-9_$]{0,63}")"#,
            "TacText.profileName((String)profile)",
        ),
        (
            r#"text.matches("[A-Za-z0-9._~-]{1,256}")"#,
            "TacText.pathSegment(text)",
        ),
        (
            r#"key.matches("[A-Z][A-Z0-9_]*")"#,
            "TacText.settingName(key)",
        ),
        (
            r#"item.getKey().matches("[a-z][a-z0-9]*")"#,
            "TacText.lowerWord(item.getKey())",
        ),
        (
            r#"value.matches("-?(0|[1-9][0-9]*)")"#,
            "TacText.integerText(value)",
        ),
        (
            r#"value.matches("-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?")"#,
            "TacText.numberText(value)",
        ),
        (
            r#"name.matches("[!#$%&'*+.^_`|~0-9A-Za-z-]{1,128}")"#,
            "TacText.headerName(name)",
        ),
        (r#"path.split("/", -1)"#, "TacText.parts(path, '/')"),
        (
            r#"((String)item).split(":", -1)"#,
            "TacText.parts((String)item, ':')",
        ),
        (
            r#"String.format(java.util.Locale.ROOT, "\\u%04x", (int)ch)"#,
            "TacText.unicodeEscape(ch)",
        ),
    ];

    pub(super) fn adapt(lowered: &str) -> String {
        let browser = crate::behavior::JavaBuiltins::in_browser(lowered, Self::BUILTINS);
        Self::TEA_VM_SWAPS
            .iter()
            .chain(&Self::REGEX_FREE_SWAPS)
            .fold(browser, |text, (missing, present)| {
                text.replace(missing, present)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::JavaBrowserRuntime;

    #[test]
    fn the_browser_runtime_keeps_no_regular_expression_or_format_call() {
        let runtime = include_str!("../../behavior/polyglot/java_runtime.java");
        let adapted = JavaBrowserRuntime::adapt(runtime);
        for costly in [".matches(", ".split(", "String.format(", "replaceAll("] {
            assert!(
                !adapted.contains(costly),
                "{costly} is still in the runtime"
            );
        }
    }
}
