#![allow(clippy::expect_used)]

use std::path::Path;

fn codes(path: &str, source: &str) -> Vec<u16> {
    crate::annotation_layout::validate(Path::new(path), source)
        .err()
        .map(|failure| {
            failure
                .diagnostics()
                .iter()
                .map(|found| found.code.number())
                .collect()
        })
        .unwrap_or_default()
}

fn messages(path: &str, source: &str) -> Vec<String> {
    crate::annotation_layout::validate(Path::new(path), source)
        .err()
        .map(|failure| {
            failure
                .diagnostics()
                .iter()
                .map(|found| found.message.clone())
                .collect()
        })
        .unwrap_or_default()
}

const VALID: [(&str, &str); 12] = [
    (
        "client/utils/TextUtils.js",
        "export default class TextUtils {\n  static slug(value) {\n    return TextUtils.#lower(value)\n  }\n\n  static #lower(value) {\n    return value.toLowerCase()\n  }\n}\n",
    ),
    (
        "server/utils/TextUtils.ts",
        "export default class TextUtils {\n  static slug(value: string): string {\n    return TextUtils.lower(value)\n  }\n\n  private static lower(value: string): string {\n    return value.toLowerCase()\n  }\n}\n",
    ),
    (
        "server/utils/TextUtils.py",
        "class TextUtils:\n    @staticmethod\n    def slug(value):\n        return TextUtils._lower(value)\n\n    @staticmethod\n    def _lower(value):\n        return value.lower()\n",
    ),
    (
        "server/utils/TextUtils.rb",
        "class TextUtils\n  def self.slug(value)\n    _lower(value)\n  end\n\n  def self._lower(value)\n    value.downcase\n  end\nend\n",
    ),
    (
        "server/utils/TextUtils.php",
        "<?php\nclass TextUtils {\n    public static function slug($value) {\n        return strtolower($value);\n    }\n}\n",
    ),
    (
        "server/utils/TextUtils.java",
        "final class TextUtils {\n    static String slug(String value) {\n        return value.toLowerCase();\n    }\n}\n",
    ),
    (
        "server/utils/TextUtils.kt",
        "object TextUtils {\n    fun slug(value: String): String = value.lowercase()\n}\n",
    ),
    (
        "server/utils/TextUtils.cs",
        "public static class TextUtils {\n    public static string Slug(string value) => value.ToLower();\n}\n",
    ),
    (
        "server/utils/TextUtils.go",
        "package main\n\ntype TextUtils struct{}\n\nfunc (TextUtils) Slug(value string) string {\n\treturn value + \"-\"\n}\n",
    ),
    (
        "server/utils/TextUtils.rs",
        "pub struct TextUtils;\n\nimpl TextUtils {\n    pub fn slug(value: &str) -> String {\n        value.to_lowercase()\n    }\n}\n",
    ),
    (
        "client/utils/TextUtils.dart",
        "class TextUtils {\n  static String slug(String value) => value.toLowerCase();\n}\n",
    ),
    (
        "client/utils/TextUtils.swift",
        "struct TextUtils {\n    static func slug(_ value: String) -> String {\n        value.lowercased()\n    }\n}\n",
    ),
];

#[test]
fn a_static_utility_class_is_valid_in_every_language() {
    for (path, source) in VALID {
        assert_eq!(messages(path, source), Vec::<String>::new(), "{path}");
    }
}

#[test]
fn an_expression_body_that_goes_on_to_the_next_lines_belongs_to_its_method() {
    let cases = [
        (
            "client/utils/TextUtils.dart",
            "class TextUtils {\n  static String slug(String value) => value\n      .toLowerCase()\n      .trim();\n\n  static String plain() => 'plain';\n}\n",
        ),
        (
            "server/utils/TextUtils.cs",
            "public static class TextUtils {\n    public static string Slug(string value) => value\n        .ToLower()\n        .Trim();\n}\n",
        ),
    ];
    for (path, source) in cases {
        assert_eq!(messages(path, source), Vec::<String>::new(), "{path}");
    }
}

#[test]
fn an_instance_method_is_refused() {
    let cases = [
        (
            "client/utils/TextUtils.js",
            "export default class TextUtils {\n  slug(value) {\n    return value\n  }\n}\n",
            "Add `static` to `slug`.",
        ),
        (
            "server/utils/TextUtils.py",
            "class TextUtils:\n    def slug(self, value):\n        return value\n",
            "Put `@staticmethod` above `slug`",
        ),
        (
            "server/utils/TextUtils.rs",
            "pub struct TextUtils;\n\nimpl TextUtils {\n    pub fn slug(&self, value: &str) -> String {\n        value.to_owned()\n    }\n}\n",
            "Remove the `self` parameter of `slug`",
        ),
        (
            "server/utils/TextUtils.rb",
            "class TextUtils\n  def slug(value)\n    value\n  end\nend\n",
            "def self.slug",
        ),
        (
            "server/utils/TextUtils.kt",
            "class TextUtils {\n    fun slug(value: String): String = value\n}\n",
            "object TextUtils",
        ),
    ];
    for (path, source, hint) in cases {
        let failure = crate::annotation_layout::validate(Path::new(path), source).expect_err(path);
        let found = &failure.diagnostics()[0];
        assert_eq!(found.code.number(), 1316, "{path}");
        assert!(
            found
                .message
                .starts_with("Method `slug` of the utility class `TextUtils` is not static"),
            "{path}: {}",
            found.message
        );
        assert!(
            found.help.as_deref().unwrap_or("").contains(hint),
            "{path}: {:?}",
            found.help
        );
    }
}

#[test]
fn a_field_or_stored_value_is_refused() {
    let cases = [
        (
            "client/utils/TextUtils.js",
            "export default class TextUtils {\n  static cache = new Map()\n\n  static slug(value) {\n    return value\n  }\n}\n",
        ),
        (
            "server/utils/TextUtils.py",
            "class TextUtils:\n    cache = {}\n\n    @staticmethod\n    def slug(value):\n        return value\n",
        ),
        (
            "server/utils/TextUtils.go",
            "package main\n\ntype TextUtils struct {\n\tcount int\n}\n\nfunc (TextUtils) Slug(value string) string {\n\treturn value\n}\n",
        ),
        (
            "server/utils/TextUtils.rs",
            "pub struct TextUtils {\n    count: u32,\n}\n\nimpl TextUtils {\n    pub fn slug(value: &str) -> String {\n        value.to_owned()\n    }\n}\n",
        ),
        (
            "server/utils/TextUtils.rb",
            "class TextUtils\n  def self.slug(value)\n    @last = value\n  end\nend\n",
        ),
        (
            "server/utils/TextUtils.java",
            "final class TextUtils {\n    static int count = 0;\n\n    static String slug(String value) {\n        return value;\n    }\n}\n",
        ),
    ];
    for (path, source) in cases {
        let found = messages(path, source);
        assert_eq!(codes(path, source), vec![1316], "{path}: {found:?}");
        assert!(
            found[0].contains("holds a field or a stored value"),
            "{path}: {found:?}"
        );
    }
}

#[test]
fn script_utilities_refuse_same_line_state_before_or_between_methods() {
    let cases = [
        "class TextUtils { static cache = 1; static text() { return 'Ready'; } }",
        "class TextUtils { static text() { return 'Ready'; } static cache = 1; static more() { return 'Ready'; } }",
        "class TextUtils { static cache = 1; static async text() { return 'Ready'; } }",
        "class TextUtils { static cache = 1; static get text() { return 'Ready'; } }",
        "class TextUtils { static cache = 1; private static text(): string { return 'Ready'; } }",
    ];
    for source in cases {
        assert_eq!(
            codes("client/utils/TextUtils.ts", source),
            vec![1316],
            "{source}"
        );
    }
    assert_eq!(codes("client/utils/TextUtils.js", cases[0]), vec![1316]);
}

#[test]
fn script_utility_prefixes_preserve_multiple_methods_without_hiding_instance_methods() {
    for source in [
        "class TextUtils { static text() { return TextUtils.#lower(); } static #lower() { return 'Ready'; } }",
        "class TextUtils { static get text() { return 'Ready'; } static set label(value) { void value; } }",
        "class TextUtils { static async text() { return 'Ready'; } static *lines() { yield 'Ready'; } }",
        "class TextUtils {\n static\n async text() { return 'Ready'; }\n static more() { return 'Ready'; }\n}",
    ] {
        assert_eq!(
            codes("client/utils/TextUtils.js", source),
            Vec::<u16>::new(),
            "{source}"
        );
    }
    let source =
        "class TextUtils { static text() { return 'Ready'; } plain() { return 'Ready'; } }";
    let found = messages("client/utils/TextUtils.js", source);
    assert_eq!(codes("client/utils/TextUtils.js", source), vec![1316]);
    assert!(found[0].contains("Method `plain`"));
    let typed = "class TextUtils { public static text(): string { return TextUtils.lower(); } private static lower(): string { return 'Ready'; } }";
    assert_eq!(codes("client/utils/TextUtils.ts", typed), Vec::<u16>::new());
}

#[test]
fn rust_utility_boundary_accepts_immutable_associated_constants() {
    let source = "pub struct TitleUtils;\nimpl TitleUtils {\n    const TITLE: &'static str = \"Ready\";\n    pub(crate) const WIDTH: usize = { 2 + 3 };\n    pub fn text() -> String { Self::TITLE.to_owned() }\n}\n";
    assert_eq!(
        messages("client/utils/TitleUtils.rs", source),
        Vec::<String>::new()
    );
}

#[test]
fn rust_utility_boundary_still_refuses_instance_and_mutable_state() {
    let cases = [
        "pub struct TitleUtils { count: usize }\nimpl TitleUtils { const TITLE: &'static str = \"Ready\"; pub fn text() -> String { Self::TITLE.to_owned() } }\n",
        "pub struct TitleUtils;\nimpl TitleUtils { static mut COUNT: usize = 0; pub fn text() -> String { String::new() } }\n",
        "pub struct TitleUtils;\nimpl TitleUtils { static COUNT: usize = 0; const TITLE: &'static str = \"Ready\"; pub fn text() -> String { Self::TITLE.to_owned() } }\n",
        "pub struct TitleUtils;\nimpl TitleUtils { const TITLE: &'static str = \"Ready\"; pub fn text(&self) -> String { Self::TITLE.to_owned() } }\n",
    ];
    for source in cases {
        assert_eq!(
            codes("client/utils/TitleUtils.rs", source),
            vec![1316],
            "{source}"
        );
    }
}

#[test]
fn rust_utility_boundary_does_not_allow_top_level_stored_values() {
    let source = "static COUNT: usize = 0;\npub struct TitleUtils;\nimpl TitleUtils { pub fn text() -> String { String::new() } }\n";
    assert!(!codes("client/utils/TitleUtils.rs", source).is_empty());
}

#[test]
fn rust_utility_boundary_refuses_function_local_persistent_state() {
    let source = "pub struct TitleUtils;\nimpl TitleUtils {\n    pub fn count() -> usize {\n        static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);\n        COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)\n    }\n}\n";
    assert_eq!(codes("client/utils/TitleUtils.rs", source), vec![1316]);
    let mutable = "pub struct TitleUtils;\nimpl TitleUtils {\n    pub fn count() -> usize {\n        static mut COUNT: usize = 0;\n        0\n    }\n}\n";
    assert_eq!(codes("client/utils/TitleUtils.rs", mutable), vec![1316]);
    let local = "pub struct TitleUtils;\nimpl TitleUtils {\n    pub fn count() -> usize {\n        let count = 1;\n        count + 1\n    }\n}\n";
    assert_eq!(
        codes("client/utils/TitleUtils.rs", local),
        Vec::<u16>::new()
    );
    let literal = "pub struct TitleUtils;\nimpl TitleUtils {\n    pub fn title() -> &'static str {\n        // static COUNT: usize = 1;\n        let text = \"static COUNT: usize = 1;\";\n        text\n    }\n}\n";
    assert_eq!(
        codes("client/utils/TitleUtils.rs", literal),
        Vec::<u16>::new()
    );
}

#[test]
fn the_folder_the_name_and_the_single_class_are_checked() {
    let class =
        "export default class TextUtils {\n  static slug(value) {\n    return value\n  }\n}\n";
    assert_eq!(
        codes("client/utils/text/TextUtils.js", class),
        Vec::<u16>::new()
    );
    assert_eq!(
        codes("server/utils/text/TextUtils.js", class),
        Vec::<u16>::new()
    );
    assert_eq!(
        codes("client/utils/Text.js", &class.replace("TextUtils", "Text")),
        vec![1310]
    );
    assert_eq!(codes("client/utils/StringUtils.js", class), vec![1310]);
    assert_eq!(
        codes("client/utils/TextUtils.js", &format!("@Service\n{class}")),
        vec![1310]
    );
    let second = format!("{class}\nclass OtherUtils {{}}\n");
    assert_eq!(codes("client/utils/TextUtils.js", &second), vec![1310]);
}

#[test]
fn utility_methods_follow_the_cohesion_rule() {
    let source = "export default class TextUtils {\n  static slug(value) {\n    return value || ''\n  }\n}\n";
    let problems = crate::annotation_layout::CohesionCheck::problems(
        Path::new("client/utils/TextUtils.js"),
        source,
    )
    .expect("scan");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].code.number(), 1314);
    let checked = crate::annotation_layout::CohesionCheck::check_file(
        Path::new("server/utils/TextUtils.js"),
        source.as_bytes(),
        crate::annotation_layout::RuleSwitch::every_realm(),
    );
    assert!(checked.is_err());
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
    std::fs::write(path, contents).expect("file");
}

#[test]
fn every_program_of_the_realm_gets_its_utility_classes_without_an_import() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/utils/TextUtils.js",
        "export default class TextUtils {\n  static clientSlug(value) {\n    return value\n  }\n}\n",
    );
    write(
        root.path(),
        "server/utils/TextUtils.js",
        "export default class TextUtils {\n  static serverSlug(value) {\n    return value\n  }\n}\n",
    );
    write(
        root.path(),
        "server/utils/TextUtils.py",
        "class TextUtils:\n    @staticmethod\n    def slug(value):\n        return value\n",
    );
    let entries = [
        ("client/services/ShellService.js", "tac", "clientSlug"),
        ("client/tests/ShellServiceTest.js", "tac", "clientSlug"),
        ("server/controllers/MoneyController.js", "yon", "serverSlug"),
        ("server/tests/MoneyControllerTest.js", "yon", "serverSlug"),
        ("server/controllers/MoneyController.py", "yon", "def slug"),
    ];
    for (entry, environment, wanted) in entries {
        write(root.path(), entry, "");
        let composed =
            crate::annotation_modules::assemble(root.path(), Path::new(entry), "", environment)
                .expect(entry)
                .unwrap_or_default();
        assert!(composed.contains("class TextUtils"), "{entry}: {composed}");
        assert!(composed.contains(wanted), "{entry}: {composed}");
        assert!(
            !composed.contains("export default class TextUtils"),
            "{entry}: {composed}"
        );
    }
}

#[test]
fn coverage_counts_the_public_static_methods_of_a_utility_class() {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/utils/PriceUtils.js",
        "export default class PriceUtils {\n  static label(cents) {\n    return PriceUtils.#dollars(cents)\n  }\n\n  static rounded(cents) {\n    return cents\n  }\n\n  static #dollars(cents) {\n    return cents / 100\n  }\n}\n",
    );
    write(
        root.path(),
        "client/tests/PriceTestSetup.js",
        "@TestSetup\nexport default class PriceTestSetup {\n  static configure() {\n    return {}\n  }\n}\n",
    );
    write(
        root.path(),
        "client/tests/PriceUtilsTest.js",
        "@Test(PriceTestSetup)\nexport default class PriceUtilsTest {\n  async testLabel() {\n    Assert.equals(1, PriceUtils.label(100))\n  }\n}\n",
    );
    let mut calls = crate::method_coverage::RecordedCalls::default();
    calls.add(
        "client",
        "PriceUtilsTest.testLabel",
        &[String::from("PriceUtils.label")],
    );
    let report =
        crate::method_coverage::CoverageReport::build(root.path(), calls).expect("coverage");
    assert_eq!(report.covered(), 1, "{report:?}");
    let uncovered: Vec<String> = report
        .uncovered()
        .into_iter()
        .map(|found| found.method)
        .collect();
    assert_eq!(uncovered, vec!["rounded"], "{report:?}");
}

#[test]
fn rust_snake_case_utilities_preserve_suffix_identity_and_statelessness() {
    let source = "pub struct TitleUtils;\nimpl TitleUtils { const TITLE: &'static str = \"Ready\"; pub fn text() -> String { Self::TITLE.to_owned() } }\n";
    for path in [
        "client/utils/title_utils.rs",
        "server/utils/nested/TitleUtils.rs",
    ] {
        assert!(
            codes(path, source).is_empty(),
            "{path}: {:?}",
            messages(path, source)
        );
    }
    for path in [
        "client/utils/titleutils.rs",
        "client/utils/text_utils.rs",
        "client/utils/title__utils.rs",
    ] {
        assert_eq!(codes(path, source), [1310], "{path}");
    }
    assert_eq!(
        codes(
            "client/utils/title_utils.rs",
            "#[Service]\nstruct TitleUtils;"
        ),
        [1310]
    );
    assert_eq!(
        codes(
            "client/utils/title_utils.rs",
            "struct TitleUtils { count: u64 }"
        ),
        [1316]
    );
    assert_eq!(
        codes(
            "client/utils/title_utils.rs",
            "struct TitleUtils;\nimpl TitleUtils { pub fn text(&self) {} }"
        ),
        [1316]
    );
}
