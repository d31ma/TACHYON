use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;

struct WebNeedsFixture;

impl WebNeedsFixture {
    const RUST_COUNTER: &'static str = "#[Service]\n#[derive(Default)]\npub struct CounterService {\n    pub count: i64,\n}\n\n\
         impl CounterService {\n    pub fn increment(&mut self) -> i64 {\n        self.count += 1;\n        self.count\n    }\n}\n";
    const KOTLIN_COUNTER: &'static str = "@Service\nclass CounterService {\n    var count: Long = 0\n\n    \
         fun increment(): Long {\n        count += 1\n        return count\n    }\n}\n";
    const SWIFT_COUNTER: &'static str = "@Service\nfinal class CounterService {\n    var count: Int = 0\n\n    \
         func increment() -> Int {\n        count += 1\n        return count\n    }\n}\n";
    const CSHARP_COUNTER: &'static str = "[Service]\npublic sealed class CounterService {\n    public int count = 0;\n    \
         public int increment() { count += 1; return count; }\n}\n";

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or(root))
            .unwrap_or_else(|error| panic!("fixture directory: {error}"));
        std::fs::write(path, source).unwrap_or_else(|error| panic!("fixture source: {error}"));
    }

    fn source(
        language: NativeCompanion,
        extension: &str,
        (counter, tally): (&str, &str),
    ) -> Result<Option<String>, crate::Failure> {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("fixture root: {error}"));
        let services = [
            ("counter", "CounterService", counter),
            ("tally", "TallyService", tally),
        ]
        .map(|(instance, class, text)| {
            let relative = format!("client/services/{class}.{extension}");
            Self::write(root.path(), &relative, text);
            (
                String::from(instance),
                String::from(class),
                root.path().join(relative),
            )
        });
        let inputs = NativeCompanionInput::web_services(root.path(), language, &services);
        super::registry::source(&inputs, language)
    }

    fn refused(language: NativeCompanion, extension: &str, pair: (&str, &str)) -> String {
        let failure = Self::source(language, extension, pair)
            .err()
            .unwrap_or_else(|| panic!("TallyService needs CounterService and must be refused"));
        assert_eq!(failure.diagnostics().len(), 1, "{failure}");
        failure.to_string()
    }

    fn assert_refused_at_field(
        language: NativeCompanion,
        extension: &str,
        (counter, tally): (&str, &str),
    ) {
        let failure = Self::source(language, extension, (counter, tally))
            .err()
            .unwrap_or_else(|| panic!("TallyService needs CounterService and must be refused"));
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.to_string(), "TY1406", "{failure}");
        assert!(
            diagnostic
                .message
                .contains("TallyService needs CounterService, another Tac service"),
            "{failure}"
        );
        assert!(
            diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help
                    .contains("JavaScript, TypeScript, Go, Python, Dart, Java, C++ or F#")),
            "{failure}"
        );
        let span = &diagnostic.spans[0];
        assert_eq!(
            span.file,
            format!("client/services/TallyService.{extension}"),
            "{failure}"
        );
        let start = usize::try_from(span.start).unwrap_or_default();
        let end = usize::try_from(span.end).unwrap_or_default();
        assert_eq!(&tally[start..end], "CounterService", "{failure}");
    }
}

#[test]
fn a_rust_browser_service_that_needs_another_service_is_refused_at_the_field() {
    let tallies = [
        "#[Service]\n#[derive(Default)]\npub struct TallyService {\n    #[Needs]\n    counter: std::sync::Arc<CounterService>,\n    pub total: i64,\n}\n\n\
         impl TallyService {\n    pub fn bump(&mut self) -> i64 {\n        0\n    }\n}\n",
        "#[Service]\n#[derive(Default)]\npub struct TallyService {\n    #[Needs]\n    counter: CounterService,\n    pub total: i64,\n}\n\n\
         impl TallyService {\n    pub fn bump(&mut self) -> i64 {\n        0\n    }\n}\n",
        "#[Service]\npub struct TallyService {\n    counter: Arc<CounterService>,\n    pub total: i64,\n}\n\n\
         impl TallyService {\n    pub fn bump(&mut self) -> i64 {\n        0\n    }\n}\n",
    ];
    for tally in tallies {
        WebNeedsFixture::assert_refused_at_field(
            NativeCompanion::Rust,
            "rs",
            (WebNeedsFixture::RUST_COUNTER, tally),
        );
    }
}

#[test]
fn a_rust_browser_service_that_takes_another_service_in_its_constructor_is_refused() {
    let tally = "#[Service]\npub struct TallyService {\n    pub total: i64,\n}\n\n\
                 impl TallyService {\n    fn new(counter: Arc<CounterService>) -> Self {\n        Self { total: counter.count }\n    }\n\n    \
                 pub fn bump(&mut self) -> i64 {\n        0\n    }\n}\n";
    let failure = WebNeedsFixture::refused(
        NativeCompanion::Rust,
        "rs",
        (WebNeedsFixture::RUST_COUNTER, tally),
    );
    assert!(
        failure.contains("TY1406") && failure.contains("its own copy of that service"),
        "{failure}"
    );
}

#[test]
fn a_kotlin_browser_service_that_needs_another_service_is_refused_at_the_field() {
    for field in [
        "@Needs\n    private val counter: CounterService",
        "@Needs private lateinit var counter: CounterService",
        "val counter: CounterService",
    ] {
        let tally = format!(
            "@Service\nclass TallyService {{\n    {field}\n    var total: Long = 0\n\n    \
             fun bump(): Long {{\n        total = counter.increment()\n        return total\n    }}\n}}\n"
        );
        WebNeedsFixture::assert_refused_at_field(
            NativeCompanion::Kotlin,
            "kt",
            (WebNeedsFixture::KOTLIN_COUNTER, &tally),
        );
    }
}

#[test]
fn a_swift_browser_service_that_needs_another_service_is_refused_at_the_field() {
    for field in [
        "@Needs\n    private let counter: CounterService",
        "@Needs var counter: CounterService!",
        "let counter: CounterService",
    ] {
        let tally = format!(
            "@Service\nfinal class TallyService {{\n    {field}\n    var total: Int = 0\n\n    \
             func bump() -> Int {{\n        total = counter.increment()\n        return total\n    }}\n}}\n"
        );
        WebNeedsFixture::assert_refused_at_field(
            NativeCompanion::Swift,
            "swift",
            (WebNeedsFixture::SWIFT_COUNTER, &tally),
        );
    }
}

#[test]
fn a_csharp_browser_service_that_needs_another_service_is_refused_with_ty1308() {
    let tally = "[Service]\npublic sealed class TallyService {\n    [Needs]\n    private readonly CounterService _counter;\n    \
                 public int bump() { return _counter.increment(); }\n}\n";
    let failure = WebNeedsFixture::refused(
        NativeCompanion::CSharp,
        "cs",
        (WebNeedsFixture::CSHARP_COUNTER, tally),
    );
    assert!(
        failure.contains("TY1308")
            && failure.contains("C# browser Needs dependency CounterService requires an assembled canonical HTTPClient"),
        "{failure}"
    );
}

#[test]
fn browser_services_that_build_their_own_values_are_not_refused() {
    let rust = "#[Service]\n#[derive(Default)]\npub struct TallyService {\n    pub total: i64,\n}\n\n\
                impl TallyService {\n    pub fn bump(&mut self) -> i64 {\n        self.total += 1;\n        self.total\n    }\n}\n";
    let kotlin = "@Service\nclass TallyService {\n    var total: Long = 0\n\n    \
                  fun bump(): Long {\n        total += 1\n        return total\n    }\n}\n";
    let swift = "@Service\nfinal class TallyService {\n    var total: Int = 0\n\n    \
                 func bump() -> Int {\n        total += 1\n        return total\n    }\n}\n";
    for (language, extension, pair) in [
        (
            NativeCompanion::Rust,
            "rs",
            (WebNeedsFixture::RUST_COUNTER, rust),
        ),
        (
            NativeCompanion::Kotlin,
            "kt",
            (WebNeedsFixture::KOTLIN_COUNTER, kotlin),
        ),
        (
            NativeCompanion::Swift,
            "swift",
            (WebNeedsFixture::SWIFT_COUNTER, swift),
        ),
    ] {
        let source = WebNeedsFixture::source(language, extension, pair)
            .unwrap_or_else(|failure| panic!("{extension}: {failure}"));
        assert!(
            source.is_some_and(|text| text.contains("TallyService")),
            "{extension}"
        );
    }
}

#[test]
fn a_rust_browser_delegate_cannot_take_on_a_service() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("fixture root: {error}"));
    WebNeedsFixture::write(
        root.path(),
        "client/services/CounterService.rs",
        WebNeedsFixture::RUST_COUNTER,
    );
    WebNeedsFixture::write(
        root.path(),
        "client/delegates/CountDelegate.rs",
        "#[Delegate]\n#[derive(Default)]\npub struct CountDelegate {\n    #[Needs]\n    counter_service: std::sync::Arc<CounterService>,\n}\n\n\
         impl CountDelegate {\n    pub fn twice(&self) -> i64 {\n        2\n    }\n}\n",
    );
    let services = [(
        String::from("counter"),
        String::from("CounterService"),
        root.path().join("client/services/CounterService.rs"),
    )];
    let inputs = NativeCompanionInput::web_services(root.path(), NativeCompanion::Rust, &services);
    let failure = super::registry::source(&inputs, NativeCompanion::Rust)
        .err()
        .unwrap_or_else(|| panic!("CountDelegate takes on CounterService and must be refused"));
    let text = failure.to_string();
    assert!(text.contains("TY1406"), "{text}");
    assert!(
        text.contains("CountDelegate takes on CounterService, but a Rust browser service cannot need another service"),
        "{text}"
    );
    assert!(text.contains("client/delegates/CountDelegate.rs"), "{text}");
}
