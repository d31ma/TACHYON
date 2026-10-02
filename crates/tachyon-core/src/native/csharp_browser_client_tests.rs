use super::csharp_browser_factory::CsharpBrowserFactory;
use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;

struct CsharpClientFixture;

impl CsharpClientFixture {
    const CLIENT: &'static str = "[HTTPClient({\"path\":\"/api/probe\",\"setup\":\"ProbeSetup\"})]\npublic interface ProbeClient {\n [GetRequest]\n System.Threading.Tasks.Task<BehaviorJSON> read();\n}\n";
    const SETUP: &'static str = "[Setup]\npublic sealed class ProbeSetup {\n public static BehaviorAdapters configure(BehaviorExecution execution) { execution.Check(); return new BehaviorAdapters(); }\n}\n";
    const SERVICE: &'static str = "[Service]\npublic sealed class ProbeService {\n [Needs]\n private readonly ProbeClient _first;\n [Needs]\n private readonly ProbeClient _second;\n public string read() { return \"ready\"; }\n}\n";

    fn write(root: &Path, relative: &str, source: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap_or(root))
            .unwrap_or_else(|error| panic!("fixture directory: {error}"));
        std::fs::write(path, source).unwrap_or_else(|error| panic!("fixture source: {error}"));
    }

    fn source(service: &str, other: Option<&str>) -> Result<String, crate::Failure> {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("fixture root: {error}"));
        Self::write(root.path(), "client/services/ProbeService.cs", service);
        Self::write(root.path(), "client/remotes/ProbeClient.cs", Self::CLIENT);
        Self::write(root.path(), "client/setups/ProbeSetup.cs", Self::SETUP);
        Self::write_other(root.path(), other);
        let services = [(
            String::from("probe"),
            String::from("ProbeService"),
            root.path().join("client/services/ProbeService.cs"),
        )];
        let inputs =
            NativeCompanionInput::web_services(root.path(), NativeCompanion::CSharp, &services);
        super::registry::source(&inputs, NativeCompanion::CSharp)
            .map(|source| source.unwrap_or_else(|| panic!("browser Service source")))
    }

    fn write_other(root: &Path, source: Option<&str>) {
        if let Some(source) = source {
            Self::write(root, "client/remotes/OtherClient.cs", source);
        }
    }
}

#[test]
fn csharp_browser_clients_use_typed_unique_activation_providers() {
    let source = CsharpClientFixture::source(CsharpClientFixture::SERVICE, None)
        .unwrap_or_else(|error| panic!("typed client assembly: {error}"));
    assert!(source.contains("new ProbeService(__tacClient0, __tacClient0)"));
    assert_eq!(
        source
            .matches("var __tacClient0 = new ProbeClient();")
            .count(),
        1
    );
    assert!(source.contains("typeof(ProbeSetup)"));
    assert!(source.contains("BehaviorBrowserHttp.Adapter(dispose)"));
    assert!(!source.contains("[\"first\"]"));
    assert!(!source.contains("[\"second\"]"));
    assert!(!source.contains("[\"_first\"]"));
    assert!(!source.contains("[\"_second\"]"));
    assert!(!source.contains("Activator.CreateInstance"));
}

#[test]
fn csharp_browser_clients_preserve_declared_constructor_order() {
    let service =
        CsharpClientFixture::SERVICE.replace("ProbeClient _second", "OtherClient _second");
    let other = CsharpClientFixture::CLIENT.replace("ProbeClient", "OtherClient");
    let source = CsharpClientFixture::source(&service, Some(&other))
        .unwrap_or_else(|error| panic!("multiple typed clients: {error}"));
    assert!(source.contains("var __tacClient0 = new OtherClient();"));
    assert!(source.contains("var __tacClient1 = new ProbeClient();"));
    assert!(source.contains("new ProbeService(__tacClient1, __tacClient0)"));
}

#[test]
fn csharp_browser_clients_refuse_unbound_noncanonical_dependencies() {
    let service =
        CsharpClientFixture::SERVICE.replace("ProbeClient _second", "MissingClient _second");
    let error = CsharpClientFixture::source(&service, None)
        .err()
        .unwrap_or_else(|| panic!("unbound Client must be refused"));
    let message = error.to_string();
    assert!(message.contains("MissingClient"), "{message}");
    assert!(message.contains("canonical HTTPClient"), "{message}");
    assert!(
        message.contains("client/services/ProbeService.cs"),
        "{message}"
    );
}

#[test]
fn csharp_browser_clients_keep_existing_no_argument_services() {
    let service = "[Service]\npublic sealed class ProbeService {\n public string read() { return \"ready\"; }\n}\n";
    let source = CsharpClientFixture::source(service, None)
        .unwrap_or_else(|error| panic!("existing no-argument Service: {error}"));
    assert!(source.contains("var Instance = new ProbeService();"));
    assert!(!source.contains("var __tacClient"));
}

#[test]
fn csharp_browser_clients_keep_initialized_publisher_out_of_constructor() {
    let service = "[Service]\n[NeedsAll]\npublic sealed class ProbeService {\n private readonly ApplicationEventPublisher _publisher;\n [OnStart]\n private void start() {}\n public string read() { return \"ready\"; }\n}\n";
    let source = CsharpClientFixture::source(service, None)
        .unwrap_or_else(|error| panic!("existing initialized publisher: {error}"));
    assert!(source.contains("_publisher = new ApplicationEventPublisher();"));
    assert!(source.contains("var Instance = new ProbeService();"));
    assert!(!source.contains("var __tacClient"));
    assert!(!source.contains("[\"_publisher\"]"));
}

#[test]
fn csharp_browser_clients_refuse_more_than_five_constructor_dependencies() {
    use std::fmt::Write as _;

    let mut fields = String::new();
    for index in 0..6 {
        write!(
            fields,
            " [Needs]\n private readonly ProbeClient _client{index};\n"
        )
        .expect("test constructor fields");
    }
    let service = format!(
        "[Service]\npublic sealed class ProbeService {{\n{fields} public string read() {{ return \"ready\"; }}\n}}\n"
    );
    let error = CsharpClientFixture::source(&service, None)
        .err()
        .unwrap_or_else(|| panic!("six dependencies must be refused"));
    let message = error.to_string();
    assert!(
        message.contains("at most 5 injected dependencies"),
        "{message}"
    );
    assert!(
        message.contains("client/services/ProbeService.cs"),
        "{message}"
    );
}

#[test]
fn csharp_browser_clients_preserve_table_initializer_and_refuse_boundary_drift() {
    let path = Path::new("client/services/ProbeService.cs");
    let factory =
        "var __tacClient0 = new ProbeClient();\nvar Instance = new ProbeService(__tacClient0)";
    let table = "var Instance = new ProbeService() { Label = \"given\" };";
    let emitted = CsharpBrowserFactory::table(table, Some(factory), "ProbeService", path)
        .unwrap_or_else(|error| panic!("creation boundary: {error}"));
    assert!(emitted.ends_with("new ProbeService(__tacClient0) { Label = \"given\" };"));
    for drift in [
        "var Instance = Factory();",
        "var Instance = new ProbeService(); var Instance = new ProbeService();",
    ] {
        let error = CsharpBrowserFactory::table(drift, Some(factory), "ProbeService", path)
            .err()
            .unwrap_or_else(|| panic!("creation boundary drift must be refused"));
        let message = error.to_string();
        assert!(
            message.contains("exactly one generated creation boundary"),
            "{message}"
        );
        assert!(
            message.contains("client/services/ProbeService.cs"),
            "{message}"
        );
    }
}

#[test]
fn csharp_browser_clients_leave_native_factory_unchanged() {
    let path = Path::new("client/services/ProbeService.cs");
    let item = NativeCompanionInput {
        snapshot_root: Path::new(".").to_path_buf(),
        language: NativeCompanion::CSharp,
        source: path.to_path_buf(),
        service: String::from("probe"),
        class: String::from("ProbeService"),
        web_assembly: false,
        application_provider: false,
    };
    let factory = CsharpBrowserFactory::prepare(&item, CsharpClientFixture::SERVICE, path)
        .unwrap_or_else(|error| panic!("native selection: {error}"));
    assert!(factory.is_none());
    let table = "var Instance = NativeConstruction();";
    assert_eq!(
        CsharpBrowserFactory::table(table, factory.as_deref(), "ProbeService", path)
            .unwrap_or_else(|error| panic!("native table: {error}")),
        table
    );
}
