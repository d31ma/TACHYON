use super::host::{native_io, native_tool_failure, run_tool_with, write};
use crate::Failure;
use crate::companion::go::{GoTable, masked_code};
use crate::go_source::GoFile;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) struct GoServices;

pub(crate) struct GoWebBuild {
    pub(crate) wasm: Vec<u8>,
    pub(crate) glue: Vec<u8>,
}

struct StagedService {
    package: String,
    instance: String,
    required: Vec<String>,
}

impl GoServices {
    const MODULE: &str = "tachyonservices";
    const RUNTIME_FILES: [(&str, &str); 15] = [
        ("tac/prelude.go", include_str!("prelude.go")),
        ("tac/prelude_json.go", include_str!("prelude_json.go")),
        (
            "tac/prelude_condition.go",
            include_str!("prelude_condition.go"),
        ),
        (
            "tac/prelude_web_host.go",
            include_str!("prelude_web_host.go"),
        ),
        (
            "tac/prelude_web_answer.go",
            include_str!("prelude_web_answer.go"),
        ),
        ("tac/clock.go", include_str!("web_apis/clock.go")),
        (
            "tac/secure_random.go",
            include_str!("web_apis/secure_random.go"),
        ),
        (
            "tac/connectivity.go",
            include_str!("web_apis/connectivity.go"),
        ),
        (
            "tac/page_visibility.go",
            include_str!("web_apis/page_visibility.go"),
        ),
        (
            "tac/preferences.go",
            include_str!("web_apis/preferences.go"),
        ),
        ("tac/clipboard.go", include_str!("web_apis/clipboard.go")),
        ("tac/share.go", include_str!("web_apis/share.go")),
        (
            "tac/notifications.go",
            include_str!("web_apis/notifications.go"),
        ),
        (
            "tac/geolocation.go",
            include_str!("web_apis/geolocation.go"),
        ),
        (
            "tac/browser_fakes.go",
            include_str!("web_apis/browser_fakes.go"),
        ),
    ];
    const WEB_ENTRY: &str = include_str!("prelude_web.go");
    const ENVIRONMENT: [(&str, &str); 8] = [
        ("GO111MODULE", "on"),
        ("GOFLAGS", ""),
        ("GOPROXY", "off"),
        ("GOTOOLCHAIN", "local"),
        ("GOWORK", "off"),
        ("CGO_ENABLED", "0"),
        ("GOOS", "js"),
        ("GOARCH", "wasm"),
    ];
    const BUILD_ARGUMENTS: [&str; 7] = [
        "build",
        "-trimpath",
        "-buildvcs=false",
        "-ldflags=-s -w",
        "-o",
        "services.wasm",
        ".",
    ];
    const GLUE_PATH: &str = "lib/wasm/wasm_exec.js";
    const FAILURE_CODE: u16 = 1605;
    const WEB_EVENT_FILE: &str = "tac/web_event_models.go";
    const ALIASED_TYPES: [&str; 19] = [
        "ApplicationEventPublisher",
        "Clock",
        "SecureRandom",
        "Connectivity",
        "PageVisibility",
        "Preferences",
        "Clipboard",
        "Share",
        "Notifications",
        "Geolocation",
        "ConnectivityChanged",
        "VisibilityChanged",
        "PreferencesChanged",
        "ClipboardTextRead",
        "ShareFinished",
        "NotificationPermissionChanged",
        "PositionFound",
        "BrowserCallFailed",
        "BrowserFakes",
    ];

    fn type_aliases() -> String {
        let mut aliases = String::new();
        for name in Self::ALIASED_TYPES {
            let _ = write!(aliases, "type {name} = tac.{name}\n\n");
        }
        aliases
    }
}

pub(crate) async fn web_go(
    snapshot_root: &Path,
    services: &[(String, String, PathBuf)],
) -> Result<GoWebBuild, Failure> {
    let files = program_files(snapshot_root, services)?;
    let version = run_go(&["version"], None).await?;
    let cached = crate::cache::root()
        .join("go-wasm")
        .join(format!("{}.wasm", program_key(&version, &files)));
    let wasm = if let Ok(bytes) = std::fs::read(&cached) {
        bytes
    } else {
        let bytes = compile(&files, services.len()).await?;
        super::rust::store_in_cache(&cached, &bytes);
        bytes
    };
    let glue = wasm_exec_glue().await?;
    Ok(GoWebBuild { wasm, glue })
}

pub(crate) fn runtime_files() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    for (name, text) in GoServices::RUNTIME_FILES {
        files.push((PathBuf::from(name), String::from(text)));
    }
    files.push((
        PathBuf::from(GoServices::WEB_EVENT_FILE),
        crate::route_types::WebEventSources::go(),
    ));
    files
}

pub(crate) fn module_file() -> String {
    format!("module {}\n\ngo 1.27\n", GoServices::MODULE)
}

pub(crate) fn publisher_alias(package: &str) -> String {
    format!(
        "package {package}\n\nimport \"{}/tac\"\n\n{}func behaviorStorageHost() func(string) string {{\n\treturn tac.Storage\n}}\n",
        GoServices::MODULE,
        GoServices::type_aliases()
    )
}

fn program_files(
    snapshot_root: &Path,
    services: &[(String, String, PathBuf)],
) -> Result<Vec<(PathBuf, String)>, Failure> {
    let mut files = vec![
        (PathBuf::from("go.mod"), module_file()),
        (
            PathBuf::from("main.go"),
            String::from(GoServices::WEB_ENTRY),
        ),
    ];
    files.extend(runtime_files());
    let models = crate::model_sources::ModelSources::read(snapshot_root, "client", "go")?;
    let mut staged = Vec::new();
    for (index, (instance, class, source)) in services.iter().enumerate() {
        let package = format!("service{index}");
        let input = ServiceInput {
            snapshot_root,
            class,
            source,
            package: &package,
        };
        let (service, required) = service_files(&input)?;
        files.extend(service);
        for (name, text) in models.separate_files() {
            files.push((Path::new(&package).join(name), in_package(&text, &package)));
        }
        staged.push(StagedService {
            package,
            instance: instance.clone(),
            required,
        });
    }
    files.push((PathBuf::from("services.go"), registry(&staged)));
    Ok(files)
}

struct ServiceInput<'a> {
    snapshot_root: &'a Path,
    class: &'a str,
    source: &'a Path,
    package: &'a str,
}

fn read_service(source: &Path) -> Result<String, Failure> {
    let text = native_io(std::fs::read_to_string(source), source)?;
    Ok(String::from(crate::without_bom(&text)))
}

type ServiceFiles = (Vec<(PathBuf, String)>, Vec<String>);

fn service_files(input: &ServiceInput<'_>) -> Result<ServiceFiles, Failure> {
    let identity = PathBuf::from(format!("client/services/{}.go", input.class));
    let relative = input
        .source
        .strip_prefix(input.snapshot_root)
        .unwrap_or(&identity);
    let roles = crate::behavior::ServiceRoles::read(relative, &read_service(input.source)?)?;
    let authored = roles.source;
    let portable = relative.to_string_lossy().replace('\\', "/");
    let assembled =
        crate::annotation_modules::assemble(input.snapshot_root, relative, &authored, "tac")?;
    let authored = assembled.unwrap_or(authored);
    let mut events = crate::events::extract(&identity, &authored, "tac")?;
    if let Some(module) = events.as_mut() {
        crate::events::describe(module, &identity, input.snapshot_root, "client")?;
    }
    let authored = match &events {
        Some(module) => module.source.clone(),
        None => authored,
    };
    let prepared = super::go_events::entries(events.as_ref(), input.class, &identity)?;
    let lowered = crate::behavior::lower(&identity, &authored, "tac")
        .map_err(|failure| GoFile::locate(&failure, &authored))?;
    let program = crate::companion::go::without_markers(&lowered.unwrap_or(authored), &portable)?;
    let mut parts = GoFile::split(&program);
    let entry = parts.remove(0);
    let table = crate::companion::go::owner_table(&GoTable {
        source: &entry.text,
        source_path: &portable,
        class: input.class,
        excluded: &prepared.excluded,
        extra: &prepared.entries,
    })?;
    let directory = Path::new(input.package);
    let mut files = vec![
        (
            directory.join("service.go"),
            format!(
                "//line {portable}:1\n{}",
                in_package(&entry.text, input.package)
            ),
        ),
        (
            directory.join("members.go"),
            format!(
                "package {}\n\nimport \"{}/tac\"\n\n{table}",
                input.package,
                GoServices::MODULE
            ),
        ),
        (directory.join("tac.go"), publisher_alias(input.package)),
    ];
    for (index, part) in parts.into_iter().enumerate() {
        let name = format!("part{index}.go");
        files.push((directory.join(name), in_package(&part.text, input.package)));
    }
    Ok((files, roles.required))
}

pub(crate) fn in_package(text: &str, package: &str) -> String {
    let code = masked_code(text, "package.go").unwrap_or_default();
    let Some(start) = code.find(|character: char| !character.is_whitespace()) else {
        return format!("package {package}; {text}");
    };
    let rest = &code[start..];
    let Some(after_keyword) = rest.strip_prefix("package") else {
        return format!("package {package}; {text}");
    };
    let name_start =
        start + "package".len() + (after_keyword.len() - after_keyword.trim_start().len());
    let name_length = code[name_start..]
        .find(|character: char| !(character.is_alphanumeric() || character == '_'))
        .unwrap_or(code.len() - name_start);
    let mut renamed = String::from(text);
    renamed.replace_range(name_start..name_start + name_length, package);
    renamed
}

fn registry(services: &[StagedService]) -> String {
    let mut imports = format!("\t\"{}/tac\"\n", GoServices::MODULE);
    let mut registrations = String::new();
    for service in services {
        let _ = writeln!(imports, "\t\"{}/{}\"", GoServices::MODULE, service.package);
        let _ = writeln!(
            registrations,
            "\ttac.Register({:?}, {}.TacCreate, {})",
            service.instance,
            service.package,
            string_slice(&service.required)
        );
    }
    format!(
        "package main\n\nimport (\n{imports})\n\nfunc registerServices() {{\n{registrations}}}\n"
    )
}

fn string_slice(values: &[String]) -> String {
    let mut quoted = Vec::new();
    for value in values {
        quoted.push(serde_json::to_string(value).unwrap_or_default());
    }
    format!("[]string{{{}}}", quoted.join(", "))
}

fn program_key(version: &str, files: &[(PathBuf, String)]) -> String {
    let mut joined = Vec::new();
    for (name, text) in files {
        joined.extend_from_slice(name.to_string_lossy().as_bytes());
        joined.push(0);
        joined.extend_from_slice(text.as_bytes());
        joined.push(0);
    }
    super::rust::cache_key(version, &joined)
}

async fn run_go(arguments: &[&str], directory: Option<&Path>) -> Result<String, Failure> {
    run_tool_with("go", arguments, directory, &GoServices::ENVIRONMENT).await
}

async fn compile(files: &[(PathBuf, String)], packages: usize) -> Result<Vec<u8>, Failure> {
    let staged = tempfile::Builder::new()
        .prefix(".tachyon-go-wasm-")
        .tempdir()
        .map_err(|error| {
            native_tool_failure(
                GoServices::FAILURE_CODE,
                &format!("Cannot stage Go services: {error}"),
            )
        })?;
    for (name, text) in files {
        write(&staged.path().join(name), text.as_bytes())?;
    }
    run_go(&GoServices::BUILD_ARGUMENTS, Some(staged.path()))
        .await
        .map_err(|failure| with_authored_paths(&failure, packages))?;
    let output = staged.path().join("services.wasm");
    native_io(std::fs::read(&output), &output)
}

fn with_authored_paths(failure: &Failure, packages: usize) -> Failure {
    let Some(first) = failure.diagnostics().first() else {
        return failure.clone();
    };
    let mut message = first.message.clone();
    for index in 0..packages {
        message = message.replace(&format!("service{index}/client/"), "client/");
    }
    native_tool_failure(GoServices::FAILURE_CODE, &message)
}

async fn wasm_exec_glue() -> Result<Vec<u8>, Failure> {
    let root = run_go(&["env", "GOROOT"], None).await?;
    let path = Path::new(root.trim()).join(GoServices::GLUE_PATH);
    std::fs::read(&path).map_err(|error| {
        native_tool_failure(
            GoServices::FAILURE_CODE,
            &format!(
                "Cannot read Go's WebAssembly glue {}: {error}",
                path.display()
            ),
        )
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn a_file_moves_into_its_service_package() {
        assert_eq!(
            in_package("package main\n\ntype A struct{}\n", "service0"),
            "package service0\n\ntype A struct{}\n"
        );
        assert_eq!(
            in_package("// note\npackage  main // here\n", "service2"),
            "// note\npackage  service2 // here\n"
        );
        assert_eq!(
            in_package("import \"strings\"\n", "service1"),
            "package service1; import \"strings\"\n"
        );
    }

    #[tokio::test]
    async fn a_compile_error_names_the_authored_service_file() {
        let go_missing = std::process::Command::new("go")
            .arg("version")
            .output()
            .is_err();
        if go_missing {
            assert!(std::env::var_os("CI").is_none(), "CI must provide go");
            eprintln!("skipping: go is unavailable");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let path = root.path().join("client/services/BrokenService.go");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, "// @Service\ntype BrokenService struct{ Count int }\n\nfunc (broken *BrokenService) Read() int {\n\treturn missing\n}\n").expect("service");
        let services = [(String::from("broken"), String::from("BrokenService"), path)];
        let Err(failure) = web_go(root.path(), &services).await else {
            panic!("an undefined name must fail the build");
        };
        let message = &failure.diagnostics()[0].message;
        assert!(
            message.contains("client/services/BrokenService.go:5"),
            "{message}"
        );
        assert!(!message.contains("service0/"), "{message}");
    }

    #[tokio::test]
    async fn a_role_file_without_behavior_markers_builds_with_its_service() {
        let go_missing = std::process::Command::new("go")
            .arg("version")
            .output()
            .is_err();
        if go_missing {
            assert!(std::env::var_os("CI").is_none(), "CI must provide go");
            eprintln!("skipping: go is unavailable");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let service = root.path().join("client/services/BillingService.go");
        let repository = root.path().join("client/stores/BillingStore.go");
        std::fs::create_dir_all(service.parent().expect("parent")).expect("services");
        std::fs::create_dir_all(repository.parent().expect("parent")).expect("stores");
        std::fs::write(&service, "type BillingService struct {\n\tTotal int\n}\n\n// @OnStart\nfunc (billing *BillingService) Start() {\n\tbilling.Total = 42\n}\n").expect("service");
        std::fs::write(&repository, "// @Store\ntype BillingStore struct{}\n\nfunc (repository *BillingStore) Subtotal() int {\n\treturn 40\n}\n").expect("repository");
        let services = [(
            String::from("billing"),
            String::from("BillingService"),
            service,
        )];
        let built = web_go(root.path(), &services)
            .await
            .expect("the role marker is removed before go build");
        assert!(built.wasm.starts_with(b"\0asm"));
    }

    #[tokio::test]
    async fn a_helper_is_refused_at_its_own_file() {
        let root = tempfile::tempdir().expect("project");
        let files = [
            (
                "client/services/TallyService.go",
                "type TallyService struct {\n\tCount int\n}\n\n// @OnStart\nfunc (tally *TallyService) Start() {\n\ttally.Count = (&StepHelper{}).Size()\n}\n",
            ),
            (
                "client/helpers/StepHelper.go",
                "import \"strings\"\n\n// @Helper\ntype StepHelper struct{}\n\nfunc (step *StepHelper) Size() int {\n\treturn len(strings.Repeat(\"a\", 2))\n}\n",
            ),
        ];
        for (relative, text) in files {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(&path, text).expect("source");
        }
        let services = [(
            String::from("tally"),
            String::from("TallyService"),
            root.path().join(files[0].0),
        )];
        let Err(failure) = web_go(root.path(), &services).await else {
            panic!("a Tac Helper is a browser role");
        };
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.to_string(), "TY1308");
        assert!(diagnostic.message.contains("browser role"), "{failure}");
        let span = &diagnostic.spans[0];
        assert_eq!(span.file, "client/helpers/StepHelper.go");
        let marker = files[1].1.find("@Helper").expect("marker");
        assert_eq!(span.start, u64::try_from(marker).expect("offset"));
    }

    #[test]
    fn the_registry_imports_and_registers_every_service() {
        let registry = registry(&[StagedService {
            package: String::from("service0"),
            instance: String::from("counter"),
            required: vec![String::from("label")],
        }]);
        assert!(registry.contains("\t\"tachyonservices/service0\"\n"));
        assert!(
            registry
                .contains("\ttac.Register(\"counter\", service0.TacCreate, []string{\"label\"})\n")
        );
    }
}
