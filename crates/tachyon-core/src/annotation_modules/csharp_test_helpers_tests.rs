use std::path::Path;

struct CsharpOwnershipProject;

impl CsharpOwnershipProject {
    const HELPER_PATH: &'static str = "client/helpers/deep/StepHelper.cs";
    const OWNERSHIP_MESSAGE: &'static str = "Tac C# Helper StepHelper needs an owner: a [Needs] field of the class that uses it, which makes a new StepHelper for that class, or an explicit caller-owned construction with new outside its own class. A Helper takes no NeedsAll, Lifetime or lifecycle hooks, and a [Needs] field names a Helper from client/helpers.";
    const ENTRY: &'static str = "client/services/TallyService.cs";
    const SERVICE: &'static str =
        "[Service]\npublic class TallyService { public string Read() { return \"ready\"; } }\n";
    const HELPER: &'static str =
        "[Helper]\npublic class StepHelper { public string Read() { return \"owned\"; } }\n";
    const NAMING_SERVICE: &'static str = "[Service]\npublic class TallyService { private readonly StepHelper step; public string Read() { return \"ready\"; } }\n";
    const TEST: &'static str = "[Test(typeof(OwnerTestSetup), {\"partition\":\"tests\"})]\npublic class TallyServiceTest {\nvoid testReady() { Assert.equals(\"ready\", new TallyService().Read()); }\n}\n";
    const SETUP: &'static str = "[TestSetup]\npublic class OwnerTestSetup { public static BehaviorAdapters configure(BehaviorExecution execution) { var owned = new StepHelper(); return new BehaviorAdapters(); } }\n";

    fn write(root: &Path, path: &str, source: &str) {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().unwrap_or_else(|| panic!("fixture parent")))
            .unwrap_or_else(|failure| panic!("fixture folder: {failure}"));
        std::fs::write(file, source).unwrap_or_else(|failure| panic!("fixture source: {failure}"));
    }

    fn project() -> tempfile::TempDir {
        let root =
            tempfile::tempdir().unwrap_or_else(|failure| panic!("ownership project: {failure}"));
        Self::write(root.path(), Self::ENTRY, Self::SERVICE);
        Self::write(
            root.path(),
            "client/helpers/deep/StepHelper.cs",
            Self::HELPER,
        );
        root
    }

    fn named() -> tempfile::TempDir {
        let root = Self::project();
        Self::write(root.path(), Self::ENTRY, Self::NAMING_SERVICE);
        root
    }

    fn setup(root: &Path, source: &str) {
        Self::write(root, "client/tests/TallyServiceTest.cs", Self::TEST);
        Self::write(root, "client/tests/OwnerTestSetup.cs", source);
    }

    fn selected_body(body: &str) -> tempfile::TempDir {
        let root = Self::named();
        let source = format!(
            "[TestSetup]\npublic class OwnerTestSetup {{ public static BehaviorAdapters configure(BehaviorExecution execution) {{ {body} }} }}\n"
        );
        Self::setup(root.path(), &source);
        root
    }

    fn selected_helper(source: &str) -> tempfile::TempDir {
        let root = Self::project();
        Self::setup(root.path(), Self::SETUP);
        Self::write(root.path(), "client/helpers/deep/StepHelper.cs", source);
        root
    }

    fn canonical_sources(root: &Path) {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for relative in [
            "call-the-server/client/services/CounterService.cs",
            "call-the-server/client/remotes/GreetingClient.cs",
            "call-the-server/client/models/Greeting.cs",
            "call-the-server/client/setups/GreetingSetup.cs",
            "add-tests/client/helpers/GreetingFakeHelper.cs",
            "add-tests/client/tests/CounterServiceTest.cs",
            "add-tests/client/tests/HelloTestSetup.cs",
        ] {
            let source = std::fs::read_to_string(
                repo.join(format!("website/content/docs/first-app/{relative}")),
            )
            .unwrap_or_else(|failure| panic!("canonical authored source: {failure}"));
            let (_, target) = relative
                .split_once("client/")
                .unwrap_or_else(|| panic!("client path"));
            Self::write(root, &format!("client/{target}"), &source);
        }
    }

    fn assembled(root: &Path) -> Result<String, crate::Failure> {
        let path = Path::new(Self::ENTRY);
        let raw = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|failure| panic!("entry source: {failure}"));
        let roles = crate::behavior::ServiceRoles::read(path, &raw)?;
        super::assemble(root, path, &roles.source, "tac")
            .map(|source| source.unwrap_or(roles.source))
    }

    fn refused_helper(root: &Path, code: u16, message: &str, location: &str) {
        let source = std::fs::read_to_string(root.join(Self::HELPER_PATH))
            .unwrap_or_else(|failure| panic!("authored Helper source: {failure}"));
        let start = source
            .find(location)
            .unwrap_or_else(|| panic!("authored refusal location: {location}"));
        let start =
            u64::try_from(start).unwrap_or_else(|failure| panic!("source offset: {failure}"));
        let failure = Self::assembled(root)
            .err()
            .unwrap_or_else(|| panic!("invalid Helper must be refused"));
        assert_eq!(failure.diagnostics().len(), 1, "{failure}");
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), code, "{failure}");
        assert_eq!(diagnostic.severity, tachyon_diagnostics::Severity::Error);
        assert_eq!(diagnostic.message, message, "{failure}");
        assert_eq!(
            diagnostic.spans,
            [tachyon_diagnostics::SourceSpan {
                file: Self::HELPER_PATH.to_owned(),
                start,
                end: start + 1,
            }],
            "{failure}"
        );
    }

    fn refused(root: &Path, message: &str) {
        let failure = Self::assembled(root)
            .err()
            .unwrap_or_else(|| panic!("invalid ownership must be refused"));
        assert!(
            failure
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.message.contains(message)),
            "{failure}"
        );
    }
}

#[test]
fn selected_csharp_test_setup_helper_is_validated_then_omitted_from_production() {
    let root = CsharpOwnershipProject::project();
    CsharpOwnershipProject::setup(root.path(), CsharpOwnershipProject::SETUP);
    let output = CsharpOwnershipProject::assembled(root.path())
        .unwrap_or_else(|failure| panic!("selected caller: {failure}"));
    assert!(!output.contains("class StepHelper"));
    assert!(!output.contains("OwnerTestSetup"));
    assert!(!output.contains("TallyServiceTest"));
    assert!(output.contains("class TallyService"));
}

#[test]
fn csharp_common_native_source_omits_selected_test_helpers() {
    let root = CsharpOwnershipProject::project();
    CsharpOwnershipProject::setup(root.path(), CsharpOwnershipProject::SETUP);
    let entry = Path::new(CsharpOwnershipProject::ENTRY);
    let roles = crate::behavior::ServiceRoles::read(entry, CsharpOwnershipProject::SERVICE)
        .unwrap_or_else(|failure| panic!("native source roles: {failure}"));
    let output = super::Module::native_source(root.path(), entry, &roles.source)
        .unwrap_or_else(|failure| panic!("native composition: {failure}"))
        .unwrap_or_else(|| panic!("native production modules"));
    assert!(output.contains("class TallyService"));
    assert!(!output.contains("class StepHelper"));
    assert!(!output.contains("OwnerTestSetup"));
}

#[test]
fn genuine_cumulative_csharp_first_app_helpers_do_not_enter_production() {
    let root = tempfile::tempdir().unwrap_or_else(|failure| panic!("first app project: {failure}"));
    CsharpOwnershipProject::canonical_sources(root.path());
    let entry = Path::new("client/services/CounterService.cs");
    let raw = std::fs::read_to_string(root.path().join(entry))
        .unwrap_or_else(|failure| panic!("Counter Service: {failure}"));
    let roles = crate::behavior::ServiceRoles::read(entry, &raw)
        .unwrap_or_else(|failure| panic!("actual Service roles: {failure}"));
    let output = super::assemble(root.path(), entry, &roles.source, "tac")
        .unwrap_or_else(|failure| panic!("post-Test production composition: {failure}"))
        .unwrap_or_else(|| panic!("production modules"));
    assert!(output.contains("class GreetingClient"));
    assert!(output.contains("class GreetingSetup"));
    assert!(!output.contains("GreetingFakeHelper"));
    assert!(!output.contains("HelloTestSetup"));
}

#[test]
fn csharp_production_and_transitive_helpers_remain_in_the_emitted_closure() {
    let root = CsharpOwnershipProject::project();
    CsharpOwnershipProject::write(
        root.path(),
        CsharpOwnershipProject::ENTRY,
        "[Service]\npublic class TallyService { private StepHelper owned = new StepHelper(); public string Read() { return owned.Read(); } }\n",
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/deep/StepHelper.cs",
        "[Helper]\npublic class StepHelper { private LeafHelper leaf = new LeafHelper(); public string Read() { return leaf.Read(); } }\n",
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/LeafHelper.cs",
        "[Helper]\npublic class LeafHelper { public string Read() { return \"leaf\"; } }\n",
    );
    let output = CsharpOwnershipProject::assembled(root.path())
        .unwrap_or_else(|failure| panic!("transitive production ownership: {failure}"));
    assert!(output.contains("class StepHelper"));
    assert!(output.contains("class LeafHelper"));
}

#[test]
fn csharp_selected_test_transitive_helpers_are_removed_together() {
    let root = CsharpOwnershipProject::project();
    CsharpOwnershipProject::setup(root.path(), CsharpOwnershipProject::SETUP);
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/deep/StepHelper.cs",
        "[Helper]\npublic class StepHelper { private LeafHelper leaf = new LeafHelper(); public string Read() { return leaf.Read(); } }\n",
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/LeafHelper.cs",
        "[Helper]\npublic class LeafHelper { public string Read() { return \"leaf\"; } }\n",
    );
    let output = CsharpOwnershipProject::assembled(root.path())
        .unwrap_or_else(|failure| panic!("selected transitive ownership: {failure}"));
    assert!(!output.contains("class StepHelper"));
    assert!(!output.contains("class LeafHelper"));
}

#[test]
fn csharp_distinct_selected_setup_types_keep_each_helper_witness() {
    let root = CsharpOwnershipProject::project();
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/LeafHelper.cs",
        "[Helper]\npublic class LeafHelper { public string Read() { return \"leaf\"; } }\n",
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/tests/first/TallyServiceTest.cs",
        CsharpOwnershipProject::TEST,
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/tests/first/OwnerTestSetup.cs",
        CsharpOwnershipProject::SETUP,
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/tests/second/OtherServiceTest.cs",
        &CsharpOwnershipProject::TEST
            .replace("TallyServiceTest", "OtherServiceTest")
            .replace("OwnerTestSetup", "SecondTestSetup"),
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/tests/second/SecondTestSetup.cs",
        &CsharpOwnershipProject::SETUP
            .replace("StepHelper", "LeafHelper")
            .replace("OwnerTestSetup", "SecondTestSetup"),
    );
    let output = CsharpOwnershipProject::assembled(root.path())
        .unwrap_or_else(|failure| panic!("separate selected constructor witnesses: {failure}"));
    assert!(!output.contains("class StepHelper"));
    assert!(!output.contains("class LeafHelper"));
    assert!(!output.contains("OwnerTestSetup"));
}

#[test]
fn csharp_helper_no_production_class_names_is_left_out_without_an_owner() {
    let root = CsharpOwnershipProject::project();
    let output = CsharpOwnershipProject::assembled(root.path())
        .unwrap_or_else(|failure| panic!("an unnamed Helper needs no owner: {failure}"));
    assert!(!output.contains("class StepHelper"));
    assert!(output.contains("class TallyService"));
}

#[test]
fn csharp_unselected_setups_cannot_establish_helper_ownership() {
    let root = CsharpOwnershipProject::named();
    CsharpOwnershipProject::write(
        root.path(),
        "client/tests/OwnerTestSetup.cs",
        CsharpOwnershipProject::SETUP,
    );
    CsharpOwnershipProject::refused(root.path(), "explicit caller-owned construction");
}

#[test]
fn csharp_unmarked_selected_setups_cannot_establish_helper_ownership() {
    let root = CsharpOwnershipProject::named();
    CsharpOwnershipProject::setup(
        root.path(),
        &CsharpOwnershipProject::SETUP.replace("[TestSetup]", ""),
    );
    CsharpOwnershipProject::refused(root.path(), "must carry TestSetup");
}

#[test]
fn csharp_comment_constructor_text_is_not_test_ownership() {
    let root = CsharpOwnershipProject::selected_body(
        "// new StepHelper();\nreturn new BehaviorAdapters();",
    );
    CsharpOwnershipProject::refused(root.path(), "explicit caller-owned construction");
}

#[test]
fn csharp_string_constructor_text_is_not_test_ownership() {
    let root = CsharpOwnershipProject::selected_body(
        "var note = \"new StepHelper()\"; return new BehaviorAdapters();",
    );
    CsharpOwnershipProject::refused(root.path(), "explicit caller-owned construction");
}

#[test]
fn csharp_helper_self_construction_is_not_test_ownership() {
    let root = CsharpOwnershipProject::named();
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/deep/StepHelper.cs",
        "[Helper]\npublic class StepHelper { private StepHelper self = new StepHelper(); }\n",
    );
    CsharpOwnershipProject::refused(root.path(), "explicit caller-owned construction");
}

#[test]
fn csharp_unrooted_helper_cycles_are_not_test_ownership() {
    let root = CsharpOwnershipProject::named();
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/deep/StepHelper.cs",
        "[Helper]\npublic class StepHelper { private LeafHelper leaf = new LeafHelper(); }\n",
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/LeafHelper.cs",
        "[Helper]\npublic class LeafHelper { private StepHelper step = new StepHelper(); }\n",
    );
    CsharpOwnershipProject::refused(root.path(), "constructor cycle is not ownership");
}

#[test]
fn csharp_test_only_helper_needs_all_remains_refused() {
    let root = CsharpOwnershipProject::selected_helper(
        "[Helper]\n[NeedsAll]\npublic class StepHelper {}\n",
    );
    CsharpOwnershipProject::refused_helper(
        root.path(),
        1308,
        CsharpOwnershipProject::OWNERSHIP_MESSAGE,
        "[NeedsAll]",
    );
}

#[test]
fn csharp_test_only_helper_lifetime_remains_refused() {
    let root = CsharpOwnershipProject::selected_helper(
        "[Helper]\n[Lifetime(\"prototype\")]\npublic class StepHelper {}\n",
    );
    CsharpOwnershipProject::refused_helper(
        root.path(),
        1308,
        CsharpOwnershipProject::OWNERSHIP_MESSAGE,
        "[Lifetime(",
    );
}

#[test]
fn csharp_test_only_helper_hook_remains_refused() {
    let root = CsharpOwnershipProject::selected_helper(
        "[Helper]\npublic class StepHelper {\n[OnStart]\npublic void Start() {}\n}\n",
    );
    CsharpOwnershipProject::refused_helper(
        root.path(),
        1308,
        CsharpOwnershipProject::OWNERSHIP_MESSAGE,
        "[OnStart]",
    );
}

#[test]
fn csharp_test_only_helper_service_base_remains_refused() {
    let root = CsharpOwnershipProject::selected_helper(
        "[Helper]\npublic class StepHelper : TallyService {}\n",
    );
    CsharpOwnershipProject::refused_helper(
        root.path(),
        1308,
        "Tac C# Helpers may implement actual interfaces, but cannot inherit behavior classes.",
        "public class StepHelper",
    );
}

#[test]
fn csharp_service_marker_under_nested_helpers_remains_refused() {
    let root = CsharpOwnershipProject::selected_helper("[Service]\npublic class StepHelper {}\n");
    CsharpOwnershipProject::refused_helper(
        root.path(),
        1310,
        "@Service type 'StepHelper' does not match its required role layout",
        "[Service]",
    );
}

#[test]
fn csharp_test_witness_does_not_permit_a_needs_all_helper() {
    let root = CsharpOwnershipProject::project();
    CsharpOwnershipProject::setup(root.path(), CsharpOwnershipProject::SETUP);
    CsharpOwnershipProject::write(
        root.path(),
        CsharpOwnershipProject::ENTRY,
        "[Service]\n[NeedsAll]\npublic class TallyService { private readonly StepHelper step; }\n",
    );
    CsharpOwnershipProject::refused(root.path(), "takes no NeedsAll");
}

#[test]
fn unrelated_invalid_tests_do_not_expand_csharp_production_discovery() {
    let root = CsharpOwnershipProject::project();
    CsharpOwnershipProject::write(
        root.path(),
        CsharpOwnershipProject::ENTRY,
        "[Service]\npublic class TallyService { private StepHelper owned = new StepHelper(); public string Read() { return owned.Read(); } }\n",
    );
    CsharpOwnershipProject::write(
        root.path(),
        "client/tests/UnrelatedServiceTest.cs",
        "BROKEN SOURCE [Test(",
    );
    let output = CsharpOwnershipProject::assembled(root.path()).unwrap_or_else(|failure| {
        panic!("production-owned Helper needs no Test witness: {failure}")
    });
    assert!(output.contains("class StepHelper"));
}

#[test]
fn unrelated_invalid_tests_do_not_expand_csharp_projects_without_helpers() {
    let root = CsharpOwnershipProject::project();
    std::fs::remove_file(root.path().join("client/helpers/deep/StepHelper.cs"))
        .unwrap_or_else(|failure| panic!("owned fixture file: {failure}"));
    CsharpOwnershipProject::write(
        root.path(),
        "client/tests/UnrelatedServiceTest.cs",
        "BROKEN SOURCE [Test(",
    );
    let output = CsharpOwnershipProject::assembled(root.path()).unwrap_or_else(|failure| {
        panic!("project without Helpers needs no Test witness: {failure}")
    });
    assert_eq!(
        output.trim(),
        "public class TallyService { public string Read() { return \"ready\"; } }"
    );
}

#[test]
fn csharp_test_helper_inheritance_failure_retains_its_authored_path() {
    let root = CsharpOwnershipProject::project();
    CsharpOwnershipProject::setup(root.path(), CsharpOwnershipProject::SETUP);
    CsharpOwnershipProject::write(
        root.path(),
        "client/helpers/deep/StepHelper.cs",
        "[Helper]\npublic class StepHelper : TallyService {}\n",
    );
    let failure = CsharpOwnershipProject::assembled(root.path())
        .err()
        .unwrap_or_else(|| panic!("class inheritance refused before pruning"));
    assert_eq!(
        failure.diagnostics()[0].spans[0].file,
        "client/helpers/deep/StepHelper.cs"
    );
}

#[test]
fn csharp_missing_caller_diagnostic_points_to_authored_helper_marker() {
    let root = CsharpOwnershipProject::named();
    CsharpOwnershipProject::refused_helper(
        root.path(),
        1308,
        CsharpOwnershipProject::OWNERSHIP_MESSAGE,
        "[Helper]",
    );
}

#[test]
fn csharp_caller_lookup_preserves_unrelated_failure_diagnostics() {
    let path = Path::new(CsharpOwnershipProject::ENTRY);
    let source = "[Helper]\npublic class StepHelper {}\n[Helper]\npublic class LeafHelper {\npublic LeafHelper(BehaviorContext context) {}\n[OnStart]\npublic void Start() {}\n}\n[Service]\npublic class TallyService { public object Read() { return new LeafHelper(null); } }\n";
    let expected = crate::yon::ManualHelpers::read(path, source)
        .err()
        .unwrap_or_else(|| panic!("unrelated caller-owned lifecycle must be refused"));
    let failure = crate::behavior::HelperRole::require_callers(path, source)
        .err()
        .unwrap_or_else(|| panic!("caller lookup must retain unrelated failure"));
    assert_eq!(failure, expected);
    assert_eq!(failure.diagnostics().len(), 1);
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 2015);
    assert_eq!(diagnostic.severity, tachyon_diagnostics::Severity::Error);
    assert_eq!(
        diagnostic.message,
        "Helper LeafHelper uses caller-owned construction; lifecycle and event hooks require a declared Needs or NeedsAll provider"
    );
    let start = source
        .find("[OnStart]")
        .unwrap_or_else(|| panic!("authored unrelated hook"));
    let start = u64::try_from(start).unwrap_or_else(|failure| panic!("hook offset: {failure}"));
    assert_eq!(
        diagnostic.spans,
        [tachyon_diagnostics::SourceSpan {
            file: CsharpOwnershipProject::ENTRY.to_owned(),
            start,
            end: start + 1,
        }]
    );
}
