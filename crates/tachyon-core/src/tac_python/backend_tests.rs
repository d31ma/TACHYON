use super::PythonWebServices;
use super::python_graph_guard::PythonGraphGuard;
use super::python_provider_source::PythonProviderSource;
use crate::behavior::{PythonMembers, ServiceRoles};
use crate::route_types::ModelTable;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

struct Project;

impl Project {
    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or(root))
            .unwrap_or_else(|error| panic!("source directory: {error}"));
        std::fs::write(path, source).unwrap_or_else(|error| panic!("source: {error}"));
    }

    fn models(root: &Path) -> ModelTable {
        ModelTable::read_realm(root, "client", "py")
            .unwrap_or_else(|error| panic!("models: {error}"))
    }

    fn provider(source: &str) -> PythonProviderSource {
        PythonProviderSource::read(Path::new("client/services/CounterService.py"), source)
            .unwrap_or_else(|error| panic!("provider: {error:?}"))
    }
}

#[test]
fn python_roles_use_the_existing_pascal_and_snake_case_spellings() {
    for roles in ["@Service\n@NeedsAll", "@service\n@needs_all"] {
        let source = format!("{roles}\nclass CounterService:\n    count: int\n");
        let parsed = ServiceRoles::read(Path::new("CounterService.py"), &source)
            .unwrap_or_else(|error| panic!("roles: {error:?}"));
        assert!(parsed.prototype);
        assert_eq!(parsed.required, ["*"]);
        assert_eq!(parsed.source.len(), source.len());
        assert!(!parsed.source.contains('@'));
        let plain = ServiceRoles::without_roles(Path::new("CounterService.py"), &source)
            .unwrap_or_else(|error| panic!("plain roles: {error:?}"));
        assert!(!plain.contains("@Service") && !plain.contains("@service\n"));
        assert!(plain.contains("NeedsAll") || plain.contains("needs_all"));
    }
    let source = "@service_alias\nclass CounterService:\n    count: int = 0\n";
    let plain = ServiceRoles::without_roles(Path::new("CounterService.py"), source)
        .unwrap_or_else(|error| panic!("unknown spelling: {error:?}"));
    assert_eq!(plain, source);
}

#[test]
fn python_members_preserve_final_fields_and_typed_instance_methods() {
    let source = "from typing import Final, ClassVar\n\nclass CounterService:\n    start_count: Final[int]\n    current_count: int = 0\n    _secret: str = 'hidden'\n    shared: ClassVar[int] = 1\n\n    def add_count(self, value: int) -> int:\n        return self.current_count + value\n\n    def _hidden(self) -> str:\n        return self._secret\n";
    let members = PythonMembers::read(Path::new("CounterService.py"), source, "CounterService")
        .unwrap_or_else(|error| panic!("members: {error:?}"));
    assert_eq!(members["fields"].as_array().map(Vec::len), Some(2));
    assert_eq!(members["fields"][0]["page"], "startCount");
    assert_eq!(members["fields"][0]["immutable"], true);
    assert_eq!(members["methods"].as_array().map(Vec::len), Some(1));
    assert_eq!(members["methods"][0]["page"], "addCount");
    assert_eq!(members["methods"][0]["arguments"], json!(["integer"]));
    assert_eq!(members["methods"][0]["returns"], "integer");
}

#[test]
fn python_registry_selects_python_and_separates_props_from_providers() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    Project::write(
        root.path(),
        "client/helpers/CounterHelper.py",
        "@helper\nclass CounterHelper:\n    def amount(self) -> int:\n        return 2\n",
    );
    Project::write(
        root.path(),
        "client/services/CounterService.py",
        "@service\n@needs_all\nclass CounterService:\n    counter: CounterHelper\n    title: str\n",
    );
    let registry = crate::template::ServiceRegistry::discover(root.path())
        .unwrap_or_else(|error| panic!("registry: {error:?}"));
    let service = registry
        .get("counter")
        .unwrap_or_else(|| panic!("counter service"));
    assert_eq!(
        service.web_assembly().map(|(language, _)| language),
        Some(crate::tac_web::WebLanguage::Python)
    );
    assert_eq!(service.required(), ["title"]);
    assert_eq!(service.injected(), ["counter"]);
}

#[test]
fn python_needs_all_has_typed_props_and_hides_injected_fields() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let provider = Project::provider(
        "@service\n@needs_all\nclass CounterService:\n    counter: CounterHelper\n    title: str\n\n    def answer(self) -> str:\n        return self.title\n",
    );
    let description = provider
        .describe(
            &BTreeSet::from(["CounterHelper".to_owned()]),
            &Project::models(root.path()),
        )
        .unwrap_or_else(|error| panic!("typed constructor: {error:?}"));
    assert_eq!(description["constructor"][0]["provider"], "CounterHelper");
    assert_eq!(description["constructor"][1]["shape"], "text");
    assert!(description["fields"].get("counter").is_none());
    assert_eq!(description["fields"]["title"]["shape"], "text");
    assert_eq!(description["methods"]["answer"]["returns"], "text");
}

#[test]
fn python_inferred_needs_cannot_become_an_unknown_page_property() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let provider = Project::provider(
        "from typing import Annotated\n\n@service\nclass CounterService:\n    counter: Annotated[MissingHelper, needs]\n",
    );
    let failure = provider
        .describe(&BTreeSet::new(), &Project::models(root.path()))
        .err()
        .unwrap_or_else(|| panic!("unresolved dependency"));
    assert!(
        failure
            .to_string()
            .contains("declared same-language provider")
    );
}

#[test]
fn python_async_and_async_lifecycle_are_explicitly_pending() {
    for source in [
        "@service\nclass CounterService:\n    async def answer(self) -> int:\n        return 1\n",
        "@service\nclass CounterService:\n    @on_start\n    async def _start(self):\n        pass\n",
    ] {
        let failure =
            PythonProviderSource::read(Path::new("client/services/CounterService.py"), source)
                .err()
                .unwrap_or_else(|| panic!("async continuation is pending"));
        assert_eq!(failure.diagnostics()[0].code.number(), 1605);
        assert!(
            failure.to_string().contains("async")
                || failure.to_string().contains("synchronous OnStart")
        );
    }
}

#[test]
fn python_unknown_method_models_fail_before_runtime() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let provider = Project::provider(
        "@service\nclass CounterService:\n    def accept(self, value: MissingModel) -> MissingModel:\n        return value\n",
    );
    let failure = provider
        .describe(&BTreeSet::new(), &Project::models(root.path()))
        .err()
        .unwrap_or_else(|| panic!("unknown DTO"));
    assert!(failure.to_string().contains("MissingModel"));
}

#[test]
fn python_graph_rejects_cycles_and_excessive_prototype_expansion() {
    let cycle = json!({"A": {"constructor": [{"provider": "B"}]}, "B": {"constructor": [{"provider": "A"}]}});
    assert!(PythonGraphGuard::check(cycle.as_object().unwrap_or_else(|| panic!("graph"))).is_err());
    let mut graph = serde_json::Map::new();
    graph.insert("Leaf".to_owned(), json!({"constructor": []}));
    for level in 0..13 {
        let child = if level == 0 {
            "Leaf".to_owned()
        } else {
            format!("Level{}", level - 1)
        };
        graph.insert(
            format!("Level{level}"),
            json!({"constructor": [{"provider": child}, {"provider": child}]}),
        );
    }
    let failure = PythonGraphGuard::check(&graph)
        .err()
        .unwrap_or_else(|| panic!("bounded graph"));
    assert!(failure.to_string().contains("4096"));
}

#[test]
fn python_program_uses_real_model_namespaces_without_executing_constructors() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    Project::write(
        root.path(),
        "client/models/Guide.py",
        "from dataclasses import dataclass\n\n@dataclass\nclass Guide:\n    title: str\n    pages: int\n",
    );
    let relative = "client/services/GuideService.py";
    Project::write(
        root.path(),
        relative,
        "from __future__ import annotations\nfrom models import Guide\n\n@service\nclass GuideService:\n    def echo(self, value: Guide) -> Guide:\n        return value\n",
    );
    let program = PythonWebServices::program(
        root.path(),
        &[(
            "guide".to_owned(),
            "GuideService".to_owned(),
            root.path().join(relative),
        )],
    )
    .unwrap_or_else(|error| panic!("compiled source: {error:?}"));
    assert!(
        program
            .trim_start()
            .starts_with("from __future__ import annotations")
    );
    assert!(program.contains("from models import Guide"));
    assert!(program.contains("_TacPythonRuntime.configure"));
    assert!(program.contains("\\\"model\\\":\\\"Guide\\\""));
    assert!(!program.contains("@service"));
    assert_eq!(
        std::fs::read_to_string(root.path().join(relative))
            .unwrap_or_else(|error| panic!("authored: {error}")),
        "from __future__ import annotations\nfrom models import Guide\n\n@service\nclass GuideService:\n    def echo(self, value: Guide) -> Guide:\n        return value\n"
    );
}

#[test]
fn python_missing_opt_in_profile_is_reported_by_doctor_without_installing() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    Project::write(
        root.path(),
        "client/services/CounterService.py",
        "@service\nclass CounterService:\n    count: int = 0\n",
    );
    let row = super::RuntimeDoctor::for_project(root.path())
        .unwrap_or_else(|error| panic!("doctor: {error:?}"))
        .unwrap_or_else(|| panic!("missing-profile row"));
    assert!(matches!(
        row.state,
        crate::doctor::ToolchainState::Incomplete(_)
    ));
    assert!(!root.path().join(".tachyon").exists());
    assert!(
        PythonWebServices::build(
            root.path(),
            &[(
                "counter".to_owned(),
                "CounterService".to_owned(),
                PathBuf::from("client/services/CounterService.py")
            )]
        )
        .is_err()
    );
}
