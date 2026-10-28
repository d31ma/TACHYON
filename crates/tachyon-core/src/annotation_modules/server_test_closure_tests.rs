#![allow(clippy::expect_used)]

use super::*;

struct ServerTestProject {
    root: tempfile::TempDir,
    source: String,
}

impl ServerTestProject {
    const ENTRY: &'static str = "server/tests/CatalogServiceTest.go";

    fn new(body: &str) -> Self {
        let project = Self {
            root: tempfile::tempdir().expect("project"),
            source: format!(
                "// @Test(CatalogTestSetup)\ntype CatalogServiceTest struct {{\nbehaviors *BehaviorContext\n}}\nfunc (test *CatalogServiceTest) TestReads() {{\nAssert.IsTrue(true)\n{body}\n}}\n"
            ),
        };
        project.write(Self::ENTRY, &project.source);
        project.write(
            "server/tests/CatalogTestSetup.go",
            "// @TestSetup\ntype CatalogTestSetup struct{}\nfunc (CatalogTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) { return NewBehaviorConfiguration(), nil }\n",
        );
        project
    }

    fn write(&self, path: &str, source: &str) {
        let path = self.root.path().join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("source");
    }

    fn assembled(&self) -> String {
        assemble(
            self.root.path(),
            Path::new(Self::ENTRY),
            &self.source,
            "yon",
        )
        .expect("assemble")
        .expect("modules")
    }

    fn unrelated(&self) {
        for index in 0..129 {
            let (folder, role, suffix) = match index % 3 {
                0 => ("controllers", "RequestRoute(\"/unrelated\")", "Controller"),
                1 => ("delegates", "Delegate", "Delegate"),
                _ => ("services", "Service", "Service"),
            };
            let name = format!("Unrelated{index:03}{suffix}");
            self.write(
                &format!("server/{folder}/{name}.go"),
                &format!("// @{role}\n// @NeedsAll()\ntype {name} struct{{}}\n"),
            );
        }
    }

    fn chain(&self, count: usize) {
        for index in 0..count {
            let field = if index + 1 < count {
                format!("next *Required{:03}Service\n", index + 1)
            } else {
                String::new()
            };
            self.write(
                &format!("server/services/Required{index:03}Service.go"),
                &format!(
                    "// @Service\n// @NeedsAll()\ntype Required{index:03}Service struct {{\n{field}}}\n"
                ),
            );
        }
    }

    fn dynamic_import(import: &str, body: &str) {
        let mut project = Self::new(body);
        project.source = format!("{import}{}", project.source);
        project.write(Self::ENTRY, &project.source);
        project.unrelated();
        project.dynamic_failure();
    }

    fn dynamic_failure(&self) {
        let output = self.assembled();
        assert_eq!(output.matches("type Unrelated").count(), 129);
        let failure = crate::yon::prepare_required_args(Path::new(Self::ENTRY), &output)
            .expect_err("required dynamic candidates exceed the managed limit");
        assert_eq!(failure.diagnostics()[0].code.number(), 2015);
        assert!(
            failure.diagnostics()[0]
                .message
                .contains("at most 128 managed providers")
        );
    }

    fn gateway(&self) {
        self.write(
            "server/delegates/GatewayOnlyDelegate.go",
            "// @Delegate\ntype GatewayOnlyDelegate struct{}\n// @Gateway({\"command\":[\"python3\"]})\nfunc (helper *GatewayOnlyDelegate) Send() (BehaviorValue, error) { return nil, nil }\n",
        );
    }
}

#[test]
fn a_go_server_test_selects_a_small_transitive_closure_before_coverage_and_constructors() {
    let project = ServerTestProject::new(
        "_ = &CatalogService{}\n// Unrelated000Controller\n_ = \"Unrelated001Delegate\"",
    );
    project.unrelated();
    project.write(
        "server/services/CatalogService.go",
        "// @Service\n// @NeedsAll()\ntype CatalogService struct {\ncounter *CounterDelegate\n}\nfunc (service *CatalogService) Read() int { return service.counter.Count() }\n",
    );
    project.write(
        "server/delegates/CounterDelegate.go",
        "// @Delegate\n// @NeedsAll()\ntype CounterDelegate struct{}\nfunc (helper *CounterDelegate) Count() int { return 7 }\n",
    );
    let output = project.assembled();
    for name in ["CatalogService", "CounterDelegate", "CatalogTestSetup"] {
        assert!(output.contains(&format!("type {name} struct")), "{output}");
    }
    assert!(!output.contains("type Unrelated"), "{output}");
    assert!(output.contains("server/services/CatalogService.go"));
    assert!(output.contains("behaviorCallsRecord(\"CatalogService.Read\")"));
    assert!(!output.contains("server/controllers/Unrelated000Controller.go"));
    assert!(
        crate::yon::prepare_required_args(Path::new(ServerTestProject::ENTRY), &output).is_ok()
    );
}

#[test]
fn a_go_server_test_keeps_selected_setup_fake_utility_and_method_body_dependencies() {
    let mut project = ServerTestProject::new("_ = &CatalogService{}");
    project.source = project.source.replace(
        "behaviors *BehaviorContext",
        "behaviors *BehaviorContext\n// @Fake\ncounter *CounterDelegate\n// @Fake\nstandalone *FakeOnlyDelegate",
    );
    project.write(ServerTestProject::ENTRY, &project.source);
    project.write(
        "server/tests/CatalogTestSetup.go",
        "// @TestSetup\ntype CatalogTestSetup struct{}\nfunc (CatalogTestSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) { _ = SetupDelegate{}.Read(); return NewBehaviorConfiguration(), nil }\n",
    );
    project.write(
        "server/delegates/SetupDelegate.go",
        "// @Delegate\ntype SetupDelegate struct{}\nfunc (SetupDelegate) Read() string { return TextUtils{}.Read() }\n",
    );
    project.write(
        "server/utils/TextUtils.go",
        "type TextUtils struct{}\nfunc (TextUtils) Read() string { return \"ready\" }\n",
    );
    project.write(
        "server/delegates/CounterDelegate.go",
        "// @Delegate\ntype CounterDelegate struct{}\n",
    );
    project.write(
        "server/delegates/FakeOnlyDelegate.go",
        "// @Delegate\ntype FakeOnlyDelegate struct{}\n",
    );
    project.write(
        "server/services/CatalogService.go",
        "// @Service\n// @NeedsAll()\ntype CatalogService struct {\ncounter *CounterDelegate\n}\n",
    );
    project.write(
        "server/tests/OtherTestSetup.go",
        "INVALID UNSELECTED SIBLING",
    );
    let output = project.assembled();
    for name in [
        "SetupDelegate",
        "CounterDelegate",
        "FakeOnlyDelegate",
        "TextUtils",
        "CatalogService",
    ] {
        assert!(output.contains(&format!("type {name} struct")), "{output}");
    }
    let prepared = crate::yon::prepare_required_args(Path::new(ServerTestProject::ENTRY), &output)
        .expect("constructors")
        .unwrap_or(output);
    let lowered = crate::behavior::lower(Path::new(ServerTestProject::ENTRY), &prepared, "yon")
        .expect("behavior")
        .expect("test lowering");
    assert!(lowered.contains("counter *CounterDelegate"), "{lowered}");
    assert!(
        lowered.contains("standalone *FakeOnlyDelegate"),
        "{lowered}"
    );
}

#[test]
fn a_go_server_test_keeps_implicit_registrations_and_their_dependencies() {
    let project = ServerTestProject::new("_ = &CatalogController{}");
    project.write(
        "server/models/CatalogReady.go",
        "type CatalogReady struct {\nValue string\n}\n",
    );
    for (name, folder, marker, field) in [
        (
            "CatalogController",
            "controllers",
            "RequestRoute(\"/catalog\")",
            "route *RouteDelegate",
        ),
        (
            "CatalogListenerService",
            "services",
            "Service",
            "event *EventDelegate",
        ),
        (
            "CatalogLifecycleService",
            "services",
            "Service",
            "lifecycle *LifecycleDelegate",
        ),
        (
            "CatalogErrorHandler",
            "controllers",
            "ErrorHandler",
            "failure *FailureDelegate",
        ),
        (
            "CatalogIdentityProvider",
            "controllers",
            "IdentityProvider",
            "identity *IdentityDelegate",
        ),
    ] {
        let method = match name {
            "CatalogListenerService" => {
                "// @OnEvent\nfunc (service *CatalogListenerService) Ready(value CatalogReady) {}\n"
            }
            "CatalogLifecycleService" => {
                "// @OnStart\nfunc (service *CatalogLifecycleService) Start() {}\n// @OnStop\nfunc (service *CatalogLifecycleService) Stop() {}\n"
            }
            _ => "",
        };
        project.write(
            &format!("server/{folder}/{name}.go"),
            &format!("// @{marker}\n// @NeedsAll()\ntype {name} struct {{\n{field}\n}}\n{method}"),
        );
    }
    for name in [
        "RouteDelegate",
        "EventDelegate",
        "LifecycleDelegate",
        "FailureDelegate",
        "IdentityDelegate",
    ] {
        project.write(
            &format!("server/delegates/{name}.go"),
            &format!("// @Delegate\ntype {name} struct{{}}\n"),
        );
    }
    let output = project.assembled();
    for name in [
        "CatalogListenerService",
        "CatalogLifecycleService",
        "CatalogErrorHandler",
        "CatalogIdentityProvider",
        "RouteDelegate",
        "EventDelegate",
        "LifecycleDelegate",
        "FailureDelegate",
        "IdentityDelegate",
    ] {
        assert!(output.contains(&format!("type {name} struct")), "{output}");
    }
    assert!(output.contains("CatalogReady"));
    let models = crate::model_sources::ModelSources::read(project.root.path(), "server", "go")
        .expect("model discovery")
        .separate_files();
    assert!(
        models
            .iter()
            .any(|(_, source)| source.contains("type CatalogReady struct"))
    );
}

#[test]
fn reflective_go_server_tests_keep_all_candidate_registrations() {
    let mut project = ServerTestProject::new("_ = reflect.TypeOf(&CatalogTestSetup{})");
    project.source = format!("import \"reflect\"\n{}", project.source);
    project.write(ServerTestProject::ENTRY, &project.source);
    project.unrelated();
    let output = project.assembled();
    assert_eq!(output.matches("type Unrelated").count(), 129);
    let failure = crate::yon::prepare_required_args(Path::new(ServerTestProject::ENTRY), &output)
        .expect_err("required dynamic candidate set exceeds the limit");
    assert_eq!(failure.diagnostics()[0].code.number(), 2015);
}

#[test]
fn direct_go_gateway_calls_keep_otherwise_unreferenced_gateway_registrations() {
    for body in [
        "_ = YonGateway.Call(\"GatewayOnlyDelegate.Send\", \"[]\")",
        "key := \"GatewayOnlyDelegate\" + \".Send\"\n_ = YonGateway.Call(key, \"[]\")",
        "_ = (YonGatewayClient{}).Call(\"GatewayOnlyDelegate.Send\", \"[]\")",
    ] {
        let project = ServerTestProject::new(body);
        project.gateway();
        let output = project.assembled();
        assert!(
            output.contains("type GatewayOnlyDelegate struct"),
            "{output}"
        );
        assert!(output.contains("@Gateway"), "{output}");
    }
}

#[test]
fn unresolved_go_gateway_targets_retain_candidates_and_fail_explicitly_above_the_limit() {
    for body in [
        "_ = YonGateway.Call(test.target, \"[]\")",
        "_ = (YonGatewayClient{}).Call(test.target, \"[]\")",
        "call := YonGateway.Call\n_ = call(test.target, \"[]\")",
    ] {
        let mut project = ServerTestProject::new(body);
        project.source = project.source.replace(
            "behaviors *BehaviorContext",
            "behaviors *BehaviorContext\ntarget string",
        );
        project.write(ServerTestProject::ENTRY, &project.source);
        project.gateway();
        assert!(
            project
                .assembled()
                .contains("type GatewayOnlyDelegate struct")
        );
        project.unrelated();
        project.dynamic_failure();
    }
}

#[test]
fn escaped_go_reflection_imports_keep_all_candidates() {
    for import in [
        "import r \"\\x72eflect\"\n",
        "import (\nr \"\\u0072eflect\"\n)\n",
        "import\nr \"\\x72eflect\"\n",
        "import /* leading\ncomment */ r \"\\x72eflect\"\n",
        "import r \"\\162eflect\"\n",
        "import r \"\\U00000072eflect\"\n",
    ] {
        ServerTestProject::dynamic_import(import, "_ = r.TypeOf(&CatalogTestSetup{})");
    }
}

#[test]
fn multiline_dot_go_reflection_imports_keep_all_candidates() {
    for (import, body) in [
        ("import .\n\"\\x72eflect\"\n", "_ = TypeOf(test)"),
        (
            "import . /* alias\ncomment */ \"\\u0072eflect\"\n",
            "_ = TypeOf(test)",
        ),
        ("import . // alias\n\"\\x72eflect\"\n", "_ = TypeOf(test)"),
        ("import (\n.\n\"\\x72eflect\"\n)\n", "_ = TypeOf(test)"),
        (
            "import (\n\"strings\"; . /* alias\ncomment */ \"\\x72eflect\"\n)\n",
            "_ = strings.TrimSpace(\"ready\")\n_ = TypeOf(test)",
        ),
    ] {
        ServerTestProject::dynamic_import(import, body);
    }
}

#[test]
fn raw_and_plain_aliased_go_reflection_imports_keep_all_candidates() {
    for (import, body) in [
        ("import r \"reflect\"\n", "_ = r.TypeOf(test)"),
        ("import r `reflect`\n", "_ = r.TypeOf(test)"),
        ("import r `reflect`/*note*/\n", "_ = r.TypeOf(test)"),
        ("import r `reflect`//note\n", "_ = r.TypeOf(test)"),
        ("import .\n`reflect`\n", "_ = TypeOf(test)"),
        ("import . `reflect`/*note*/\n", "_ = TypeOf(test)"),
        ("import . `reflect`//note\n", "_ = TypeOf(test)"),
        (
            "import (\n. /* alias\ncomment */ `reflect`\n)\n",
            "_ = TypeOf(test)",
        ),
        ("import _ `unsafe`\n", "Assert.IsTrue(true)"),
        ("import . \"unsafe\"\n", "_ = Sizeof(test)"),
        ("import r \"\\x75nsafe\"\n", "_ = r.Sizeof(test)"),
    ] {
        ServerTestProject::dynamic_import(import, body);
    }
}

#[test]
fn ordinary_go_body_strings_and_comments_do_not_trigger_dynamic_import_retention() {
    for (import, body) in [
        ("", "_ = \"reflect\"\n_ = `reflect`\n_ = \"unsafe\""),
        ("// import r \"reflect\"\n", "_ = \"\\x72eflect\""),
        ("/* import .\n\"\\x72eflect\" */\n", "Assert.IsTrue(true)"),
        (
            "import \"strings\"\n",
            "_ = strings.TrimSpace(\"\\tready\")",
        ),
        (
            "import\n\"strings\"\n",
            "_ = strings.TrimSpace(\"\\tready\")",
        ),
        ("import (\n.\n`strings`\n)\n", "_ = TrimSpace(\"\\tready\")"),
        (
            "import /* \\x72eflect */ \"strings\"\n",
            "_ = strings.TrimSpace(\"\\tready\")",
        ),
        (
            "import s /* \\x72eflect */ \"strings\"\n",
            "_ = s.TrimSpace(\"\\tready\")",
        ),
        (
            "import s `strings`/* \\x72eflect */\n",
            "_ = s.TrimSpace(\"\\tready\")",
        ),
    ] {
        let mut project = ServerTestProject::new(body);
        project.source = format!("{import}{}", project.source);
        project.write(ServerTestProject::ENTRY, &project.source);
        project.unrelated();
        assert!(!project.assembled().contains("type Unrelated"));
    }
}

#[test]
fn a_go_server_test_keeps_dependencies_of_every_actually_staged_shared_source() {
    let project = ServerTestProject::new("Assert.IsTrue(true)");
    project.unrelated();
    project.write(
        "server/shared/mailbox/PlainShared.go",
        "type PlainShared struct {\nservice *SharedRootService\n}\nfunc (shared PlainShared) Read() int { return shared.service.Read() }\n",
    );
    project.write(
        "server/services/SharedRootService.go",
        "// @Service\n// @NeedsAll()\ntype SharedRootService struct {\nleaf *SharedLeafDelegate\n}\nfunc (service *SharedRootService) Read() int { return service.leaf.Read() }\n",
    );
    project.write(
        "server/delegates/SharedLeafDelegate.go",
        "// @Delegate\n// @NeedsAll()\ntype SharedLeafDelegate struct{}\nfunc (helper *SharedLeafDelegate) Read() int { return 7 }\n",
    );
    project.write(
        "server/shared/ignored.txt",
        "type Unstaged struct {\ncontroller *Unrelated000Controller\n}\n",
    );
    project.write(
        "client/shared/utils/Ignored.go",
        "type Ignored struct {\nhelper *Unrelated001Delegate\n}\n",
    );
    let staged = crate::handler::GoSharedCode::read(project.root.path()).expect("shared staging");
    assert_eq!(staged.len(), 1);
    assert!(staged[0].1.contains("type PlainShared struct"));
    let output = project.assembled();
    for name in ["SharedRootService", "SharedLeafDelegate"] {
        assert!(output.contains(&format!("type {name} struct")), "{output}");
    }
    assert!(!output.contains("type Unrelated"), "{output}");
    assert!(!output.contains("type PlainShared struct"), "{output}");
    assert!(output.contains("behaviorCallsRecord(\"SharedRootService.Read\")"));
    assert!(
        crate::yon::prepare_required_args(Path::new(ServerTestProject::ENTRY), &output).is_ok()
    );
}

#[test]
fn shared_source_dependencies_still_fail_when_the_required_provider_closure_exceeds_128() {
    let project = ServerTestProject::new("Assert.IsTrue(true)");
    project.chain(129);
    project.write(
        "server/shared/SharedRoot.go",
        "type SharedRoot struct {\nservice *Required000Service\n}\n",
    );
    let output = project.assembled();
    assert_eq!(output.matches("type Required").count(), 129);
    let failure = crate::yon::prepare_required_args(Path::new(ServerTestProject::ENTRY), &output)
        .expect_err("shared sources require more than 128 managed providers");
    assert_eq!(failure.diagnostics()[0].code.number(), 2015);
    assert!(
        failure.diagnostics()[0]
            .message
            .contains("at most 128 managed providers")
    );
}

#[test]
fn generated_go_factory_names_do_not_replace_real_declaration_owners() {
    let project = ServerTestProject::new("_ = &NewCatalogService{}");
    for name in ["CatalogService", "NewCatalogService"] {
        project.write(
            &format!("server/services/{name}.go"),
            &format!("// @Service\ntype {name} struct{{}}\n"),
        );
    }
    let output = project.assembled();
    assert!(output.contains("type CatalogService struct"));
    assert!(output.contains("type NewCatalogService struct"));
}

#[test]
fn both_go_managed_provider_limits_still_reject_a_required_129_provider_chain() {
    for count in [128, 129] {
        let project = ServerTestProject::new("_ = &Required000Service{}");
        project.chain(count);
        let output = project.assembled();
        assert_eq!(output.matches("type Required").count(), count);
        let parser_source = output
            .lines()
            .filter(|line| !line.trim_start().starts_with("// @NeedsAll"))
            .collect::<Vec<_>>()
            .join("\n");
        let prepared =
            crate::yon::prepare_required_args(Path::new(ServerTestProject::ENTRY), &output);
        if count == 128 {
            assert!(prepared.is_ok(), "{prepared:?}");
            let parsed = crate::yon::parser::parse_prepared(
                Path::new(ServerTestProject::ENTRY),
                &parser_source,
                Some("Required000Service"),
                &BTreeSet::new(),
            );
            assert!(
                parsed.is_ok(),
                "{}",
                parsed
                    .err()
                    .map(|failure| failure.to_string())
                    .unwrap_or_default()
            );
            continue;
        }
        let failure = prepared.expect_err("constructor limit");
        assert_eq!(failure.diagnostics()[0].code.number(), 2015);
        assert!(
            failure.diagnostics()[0]
                .message
                .contains("at most 128 managed providers")
        );
        let failure =
            crate::yon::parser::parse_test(Path::new(ServerTestProject::ENTRY), &parser_source)
                .err()
                .expect("parser limit");
        assert_eq!(failure.diagnostics()[0].code.number(), 2015);
        assert!(
            failure.diagnostics()[0]
                .message
                .contains("at most 128 managed providers")
        );
    }
}
