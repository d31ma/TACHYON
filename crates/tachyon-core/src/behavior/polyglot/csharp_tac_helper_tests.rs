#![allow(clippy::expect_used)]

use std::path::Path;

struct CsharpTacHelperTests;

impl CsharpTacHelperTests {
    const HELPER: &'static str = "[Delegate]\npublic sealed class LabelDelegate {\n private readonly string label;\n public LabelDelegate(string label) { this.label = label; }\n public string Read() { return label; }\n}\n";
    const CALLER: &'static str = "[Setup]\npublic class LabelSetup {\n public static BehaviorAdapters configure(BehaviorExecution execution) {\n var helper = new LabelDelegate(\"owned\");\n helper.Read();\n return new BehaviorAdapters();\n }\n}\n";

    fn lower(source: &str) -> Result<Option<String>, crate::Failure> {
        let path = Path::new("client/setups/LabelSetup.cs");
        let stripped = crate::behavior::ServiceRoles::read(path, source)?.source;
        super::lower(path, &stripped, "tac")
    }
}

#[test]
fn csharp_tac_helper_preserves_explicit_setup_and_service_constructors() {
    let service = "[Service]\npublic class LabelService {\n private readonly LabelDelegate helper;\n public LabelService() { helper = new LabelDelegate(\"service\"); }\n public string Read() { return helper.Read(); }\n}\n";
    let source = format!(
        "{}{}{service}",
        CsharpTacHelperTests::HELPER,
        CsharpTacHelperTests::CALLER
    );
    let lowered = CsharpTacHelperTests::lower(&source)
        .expect("caller-owned Delegate")
        .expect("lowered source");
    assert!(!lowered.contains("[Delegate]"));
    assert!(lowered.contains("public LabelDelegate(string label)"));
    assert!(lowered.contains("new LabelDelegate(\"owned\")"));
    assert!(lowered.contains("new LabelDelegate(\"service\")"));
    assert!(!lowered.contains("construct<LabelDelegate>"));
}

#[test]
fn csharp_tac_helper_does_not_take_comments_strings_or_self_recursion_as_callers() {
    for caller in [
        "",
        "// new LabelDelegate(\"owned\");\n",
        "class Note { string text = \"new LabelDelegate()\"; }\n",
        "class Note { object Run() { return Behaviors.construct<LabelDelegate>(); } }\n",
        "class Note { object Run() { return Behaviors.construct(typeof(LabelDelegate)); } }\n",
        "class Note { object Run() { return new LabelDelegateSuffix(); } }\n",
    ] {
        let source = format!("{}{caller}", CsharpTacHelperTests::HELPER);
        let error = CsharpTacHelperTests::lower(&source).expect_err("missing explicit caller");
        assert!(
            error
                .to_string()
                .contains("explicit caller-owned construction"),
            "{error}"
        );
    }
    let recursive = "[Delegate]\nclass LabelDelegate {\n public LabelDelegate() { var recursive = new LabelDelegate(); }\n}\n";
    let error =
        CsharpTacHelperTests::lower(recursive).expect_err("self recursion is not ownership proof");
    assert!(
        error.to_string().contains("outside its own class"),
        "{error}"
    );
}

#[test]
fn csharp_tac_helper_refuses_unfilled_helper_dependencies_even_with_another_real_caller() {
    for marker in ["[Needs]", "[NeedsAll]"] {
        let service = if marker == "[Needs]" {
            "[Service]\nclass ReaderService {\n [Needs]\n private readonly LabelDelegate helper;\n}\n"
                .to_owned()
        } else {
            "[Service]\n[NeedsAll]\nclass ReaderService {\n private readonly LabelDelegate helper;\n}\n".to_owned()
        };
        let source = format!(
            "{}{}{service}",
            CsharpTacHelperTests::HELPER,
            CsharpTacHelperTests::CALLER
        );
        let error = CsharpTacHelperTests::lower(&source).expect_err("unfilled Delegate dependency");
        assert!(
            error.to_string().contains("takes no NeedsAll"),
            "{marker}: {error}"
        );
    }
}

#[test]
fn csharp_tac_helper_capture_refuses_class_ownership_and_hooks_at_authored_spans() {
    for marker in [
        "[Needs]",
        "[NeedsAll]",
        "[Lifetime({\"value\":\"prototype\"})]",
    ] {
        let source = format!("[Delegate]\n{marker}\nclass LabelDelegate {{}}\n");
        let path = Path::new("client/delegates/nested/LabelDelegate.cs");
        let error = crate::behavior::HelperRole::refuse_in_tac(path, &source)
            .expect_err("managed Delegate role");
        let diagnostic = &error.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1308);
        assert_eq!(diagnostic.spans[0].file, path.to_string_lossy());
        assert_eq!(
            diagnostic.spans[0].start,
            u64::try_from(source.find(marker).expect("marker")).expect("marker offset")
        );
    }
    for marker in ["OnStart", "OnStop", "OnEvent"] {
        let source =
            format!("[Delegate]\nclass LabelDelegate {{\n [{marker}]\n public void Run() {{}}\n}}\n");
        let error = crate::behavior::HelperRole::refuse_in_tac(
            Path::new("client/delegates/LabelDelegate.cs"),
            &source,
        )
        .expect_err("unsupported Delegate hook");
        assert!(
            error.to_string().contains("lifecycle hooks"),
            "{marker}: {error}"
        );
        assert_eq!(
            error.diagnostics()[0].spans[0].start,
            u64::try_from(source.find(&format!("[{marker}]")).expect("hook span"))
                .expect("hook offset")
        );
    }
}

#[test]
fn csharp_tac_helper_keeps_folder_and_role_guards() {
    for (path, source) in [
        ("client/delegates/LabelDelegate.cs", "class LabelDelegate {}"),
        (
            "client/helpers/LabelService.cs",
            "[Service]\nclass LabelService {}",
        ),
        (
            "client/services/LabelDelegate.cs",
            "[Delegate]\nclass LabelDelegate {}",
        ),
    ] {
        assert!(
            crate::annotation_layout::FolderRoleCheck::check(Path::new(path), source).is_err(),
            "{path}"
        );
    }
    for (path, source) in [
        (
            "client/delegates/LabelDelegate.java",
            "@Delegate\npublic class LabelDelegate {}",
        ),
        (
            "client/delegates/LabelDelegate.kt",
            "@Delegate\nclass LabelDelegate {}",
        ),
        (
            "client/delegates/LabelDelegate.swift",
            "@Delegate\nfinal class LabelDelegate {}",
        ),
    ] {
        assert!(
            crate::behavior::HelperRole::refuse_in_tac(Path::new(path), source).is_ok(),
            "{path}"
        );
    }
}

#[test]
fn csharp_tac_helper_generates_a_typed_constructor_only_for_explicit_callers() {
    let source = "[Delegate]\nclass InputDelegate {\n [Needs]\n private readonly string label;\n public string Read() { return label; }\n}\n[Setup]\nclass InputSetup {\n public static BehaviorAdapters configure(BehaviorExecution execution) {\n var helper = new InputDelegate(\"typed\");\n helper.Read();\n return new BehaviorAdapters();\n }\n}\n";
    let lowered = CsharpTacHelperTests::lower(source)
        .expect("explicit constructor request")
        .expect("lowered source");
    assert!(
        lowered.contains("public InputDelegate(string label)"),
        "{lowered}"
    );
    assert!(lowered.contains("this.label = label;"), "{lowered}");
    assert!(lowered.contains("new InputDelegate(\"typed\")"), "{lowered}");
}

#[test]
fn csharp_tac_helper_ordinary_http_fixture_retains_context_owned_composition() {
    let root = tempfile::tempdir().expect("actual HTTP fixture");
    let entry = Path::new("client/services/HttpProbeService.cs");
    let source =
        include_str!("../../../tests/fixtures/tac-dotnet-http/client/services/HttpProbeService.cs");
    for (relative, source) in [
        ("client/services/HttpProbeService.cs", source),
        (
            "client/setups/BrowserHttpSetup.cs",
            include_str!(
                "../../../tests/fixtures/tac-dotnet-http/client/setups/BrowserHttpSetup.cs"
            ),
        ),
        (
            "client/delegates/CustomHttpDelegate.cs",
            include_str!(
                "../../../tests/fixtures/tac-dotnet-http/client/delegates/CustomHttpDelegate.cs"
            ),
        ),
        (
            "client/setups/OverrideHttpSetup.cs",
            include_str!(
                "../../../tests/fixtures/tac-dotnet-http/client/setups/OverrideHttpSetup.cs"
            ),
        ),
    ] {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }
    let roles = crate::behavior::ServiceRoles::read(entry, source).expect("root roles");
    let composed = crate::annotation_modules::assemble(root.path(), entry, &roles.source, "tac")
        .expect("validated discovery")
        .expect("Delegate and Setup assembly");
    let events = crate::events::extract(entry, &composed, "tac").expect("owned root lifecycle");
    let source = events
        .as_ref()
        .map_or(composed.as_str(), |module| module.source.as_str());
    let lowered = crate::behavior::lower(entry, source, "tac")
        .expect("explicit Delegate lowering")
        .expect("runtime");
    assert!(lowered.contains("class CustomHttpDelegate : BehaviorHTTPAdapter"));
    assert!(lowered.contains("new CustomHttpDelegate()"));
    assert!(lowered.contains("context.Dispose();"));
    assert!(!lowered.contains("[Delegate]"));
}

#[test]
fn csharp_tac_helper_keeps_bare_marker_argument_validation() {
    let source = "[Delegate(\"custom\")]\nclass LabelDelegate {}\n";
    assert!(
        crate::behavior::ServiceRoles::read(Path::new("client/delegates/LabelDelegate.cs"), source)
            .is_err()
    );
}

#[test]
fn csharp_tac_helper_keeps_yon_managed_roles_and_lowering() {
    let path = Path::new("server/controllers/LabelController.cs");
    let source = "[Service]\npublic class LabelService {\n public string Read() { return \"yon\"; }\n}\n[Delegate]\n[NeedsAll]\npublic class OwnedDelegate {\n private readonly LabelService labelService;\n public string Read() { return labelService.Read(); }\n [OnStop]\n public void Close() { labelService.Read(); }\n}\n[RequestRoute(\"/label\")]\n[NeedsAll]\npublic class LabelController {\n private readonly OwnedDelegate ownedDelegate;\n [GetRoute]\n public BehaviorJSON getLabel() { return BehaviorJSON.From(ownedDelegate.Read()); }\n}\n";
    let roles = crate::behavior::ServiceRoles::read(path, source).expect("Yon role consumption");
    assert!(!roles.source.contains("[Delegate]"));
    assert!(!roles.source.contains("[NeedsAll]"));
    assert!(roles.source.contains("[OnStop]"));
    crate::behavior::HelperRole::refuse_in_tac(path, source)
        .expect("Tac adapter does not restrict Yon source");
    let lowered = crate::yon::lower_with_routes(path, source, None)
        .expect("real Yon managed provider pipeline")
        .expect("managed Delegate emission");
    assert!(lowered.source.contains("new OwnedDelegate("));
    assert!(lowered.source.contains("LabelService"));
    assert!(lowered.source.contains("labelService.Read();"));
    assert!(!lowered.source.contains("[NeedsAll]"));
}

#[test]
fn csharp_tac_helper_retains_markers_only_in_the_client_realm() {
    for path in [
        "client/delegates/LabelDelegate.cs",
        "client/shared/native/LabelDelegate.cs",
        "/owned/project/client/delegates/nested/LabelDelegate.cs",
    ] {
        assert!(
            crate::behavior::HelperRole::keeps_tac_marker(Path::new(path)),
            "{path}"
        );
    }
    for path in [
        "server/delegates/LabelDelegate.cs",
        "server/shared/LabelDelegate.cs",
        "server/client/LabelDelegate.cs",
        "LabelDelegate.cs",
        "client/delegates/LabelDelegate.kt",
    ] {
        assert!(
            !crate::behavior::HelperRole::keeps_tac_marker(Path::new(path)),
            "{path}"
        );
    }
}

#[test]
fn csharp_tac_helper_allows_only_actual_interfaces_not_base_classes() {
    let caller = CsharpTacHelperTests::CALLER;
    let interface = "public interface LabelContract { string Read(); }\n";
    let helper = CsharpTacHelperTests::HELPER
        .replace("class LabelDelegate {", "class LabelDelegate : LabelContract {");
    let source = format!("{interface}{helper}{caller}");
    let lowered = CsharpTacHelperTests::lower(&source)
        .expect("actual same-language interface")
        .expect("helper source");
    assert!(lowered.contains("class LabelDelegate : LabelContract"));
    let path = Path::new("server/delegates/LabelDelegate.cs");
    let input = super::super::Input {
        path,
        source: &source,
        code: crate::annotation_layout::masked_source(path, &source).expect("authored source mask"),
    };
    let mut ty = super::declarations(&input, "cs")
        .expect("real source declarations")
        .into_iter()
        .find(|ty| ty.name == "LabelDelegate")
        .expect("Delegate declaration");
    ty.policies
        .insert("helper".to_owned(), serde_json::Value::Bool(true));
    assert!(
        !super::csharp_helper_interfaces::CsharpHelperInterfaces::allowed(&input, "cs", &ty)
            .expect("Yon does not use the Tac interface allowance")
    );
    for base in ["BaseClass", "MissingContract", "LabelContract, BaseClass"] {
        let helper = CsharpTacHelperTests::HELPER.replace(
            "class LabelDelegate {",
            &format!("class LabelDelegate : {base} {{"),
        );
        let source = format!("{interface}public class BaseClass {{}}\n{helper}{caller}");
        let error =
            CsharpTacHelperTests::lower(&source).expect_err("no concrete or unknown inheritance");
        assert!(error.to_string().contains("cannot inherit"), "{error}");
    }
    let service = "[Service]\n[CacheSettings({\"name\":\"probe\",\"ttlMs\":1000,\"maxBytes\":65536,\"maxEntries\":16})]\nclass LabelService : LabelContract { public string Read() { return \"service\"; } }\n";
    let error = CsharpTacHelperTests::lower(&format!("{interface}{service}"))
        .expect_err("Service inheritance unchanged");
    assert!(error.to_string().contains("cannot inherit"), "{error}");
}

#[test]
fn csharp_tac_helper_hook_capture_ignores_strings_comments_and_other_owners() {
    let source = format!(
        "{}// [OnStop]\nclass Note {{ string text = \"[OnStart]\"; }}\n[Service]\nclass ReaderService {{\n [OnStart]\n public void Start() {{}}\n}}\n",
        CsharpTacHelperTests::HELPER
    );
    crate::behavior::HelperRole::refuse_in_tac(Path::new("client/delegates/LabelDelegate.cs"), &source)
        .expect("only Delegate-owned real hooks are refused");
}
