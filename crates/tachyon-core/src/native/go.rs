use super::go_compiler_choice::GoCompilerChoice;
use super::go_web_build::GoWebBuild;
use super::host::{native_io, native_tool_failure};
use crate::Failure;
use crate::companion::go::{GoTable, masked_code};
use crate::go_source::GoFile;
use crate::wasm_release_step::WasmReleaseStep;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) struct GoServices;

struct StagedService {
    package: String,
    instance: String,
    required: Vec<String>,
}

impl GoServices {
    const MODULE: &str = "tachyonservices";
    const RUNTIME_FILES: [(&str, &str); 17] = [
        ("tac/prelude.go", include_str!("prelude.go")),
        ("tac/prelude_json.go", include_str!("prelude_json.go")),
        ("tac/prelude_tasks.go", include_str!("prelude_tasks.go")),
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
            "tac/window_controls.go",
            include_str!("web_apis/window_controls.go"),
        ),
        (
            "tac/browser_fakes.go",
            include_str!("web_apis/browser_fakes.go"),
        ),
    ];
    const WEB_ENTRY: &str = include_str!("prelude_web.go");
    const FAILURE_CODE: u16 = 1605;
    const WEB_EVENT_FILE: &str = "tac/web_event_models.go";
    const ALIASED_TYPES: [&str; 20] = [
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
        "WindowControls",
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
    profile: crate::BuildProfile,
) -> Result<GoWebBuild, Failure> {
    let files = program_files(snapshot_root, services)?;
    let choice = GoCompilerChoice::read(snapshot_root, profile)?;
    let release = WasmReleaseStep::prepare(profile).await;
    GoWebBuild::load(&files, &choice, &release)
        .await
        .map_err(|failure| with_authored_paths(&failure, services.len()))
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
        "package {package}\n\nimport \"{}/tac\"\n\n{}func behaviorStorageHost() func(string) string {{\n\treturn tac.Storage\n}}\n\nfunc behaviorBrowserHost() func(string) string {{\n\treturn tac.Exchange\n}}\n",
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
    let lowered = crate::model_sources::ModelSources::client_metadata(input.snapshot_root, "go")?
        .lowered(&identity, &authored, |text| {
        crate::behavior::lower(&identity, text, "tac")
            .map_err(|failure| GoFile::locate(&failure, text))
    })?;
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

pub(super) fn program_key(version: &str, files: &[(PathBuf, String)]) -> String {
    let mut joined = Vec::new();
    for (name, text) in files {
        joined.extend_from_slice(name.to_string_lossy().as_bytes());
        joined.push(0);
        joined.extend_from_slice(text.as_bytes());
        joined.push(0);
    }
    super::rust::cache_key(version, &joined)
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    struct PageFixtures;

    impl PageFixtures {
        const WRITER: &str = r#"package main

import "strings"

// @Service
// @NeedsAll
type WriterService struct {
    Count int
    Label string
    publisher ApplicationEventPublisher
    context *BehaviorContext
    client *PageClient
}

// @OnStart
func (writer *WriterService) Boot() error {
    context, err := BehaviorsCreate[PageSetup](NewBehaviorOptions("public-writer", NewBehaviorSignal()))
    if err != nil { return err }
    writer.context = context
    writer.client = NewPageClientWith(context)
    writer.Label = strings.ToUpper("writer")
    return nil
}

// @OnStop
func (writer *WriterService) Close() { writer.context.Dispose() }

func (writer *WriterService) Inspect() error {
    state, err := writer.client.Read()
    writer.Count = state.Count
    return err
}
"#;
        const READER: &str = r#"package main

import "strconv"

// @Service
type ReaderService struct {
    Count int
    Label string
    publisher *ApplicationEventPublisher
    context *BehaviorContext
    client *PageClient
}

// @OnStart
func (reader *ReaderService) Begin() error {
    context, err := BehaviorsCreate[PageSetup](NewBehaviorOptions("public-reader", NewBehaviorSignal()))
    if err != nil { return err }
    reader.context = context
    reader.client = NewPageClientWith(context)
    reader.Label = strconv.Itoa(0)
    return nil
}

// @OnStop
func (reader *ReaderService) End() { reader.context.Dispose() }

func (reader *ReaderService) Inspect() error {
    state, err := reader.client.Read()
    reader.Count = state.Count
    return err
}
"#;
        const SETUP: &str = r#"package main

// @Setup
type PageSetup struct { Count int }

func (setup *PageSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
    configuration := NewBehaviorConfiguration()
    configuration.Http["page"] = setup
    return configuration, nil
}

func (setup *PageSetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
    if err := execution.Check(); err != nil { return nil, err }
    return PageSnapshot{Count: setup.Count}, nil
}
"#;
        const CLIENT: &str = r#"package main

// @HTTPClient({"adapter":"page","path":"/page","setup":"PageSetup"})
type PageClient interface {
    // @GetRequest("/read")
    Read() (PageSnapshot, error)
}
"#;
        const MODEL: &str = "package main\n\ntype PageSnapshot struct { Count int }\n";

        fn write(root: &Path, relative: &str, source: &str) {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, source).expect("authored source");
        }

        fn project() -> tempfile::TempDir {
            let root = tempfile::tempdir().expect("project");
            for (relative, source) in [
                ("client/services/WriterService.go", Self::WRITER),
                ("client/services/ReaderService.go", Self::READER),
                ("client/setups/PageSetup.go", Self::SETUP),
                ("client/remotes/PageClient.go", Self::CLIENT),
                ("client/models/PageSnapshot.go", Self::MODEL),
            ] {
                Self::write(root.path(), relative, source);
            }
            root
        }

        fn services(root: &Path) -> Vec<(String, String, PathBuf)> {
            ["WriterService", "ReaderService"]
                .into_iter()
                .map(|class| {
                    (
                        class.trim_end_matches("Service").to_ascii_lowercase(),
                        String::from(class),
                        root.join(format!("client/services/{class}.go")),
                    )
                })
                .collect()
        }

        fn text(files: &[(PathBuf, String)]) -> String {
            files
                .iter()
                .map(|(_, text)| text.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }

        fn file<'a>(files: &'a [(PathBuf, String)], needle: &str) -> (&'a Path, &'a str) {
            let (path, text) = files
                .iter()
                .find(|(_, text)| text.contains(needle))
                .expect(needle);
            (path.as_path(), text.as_str())
        }
    }

    #[test]
    fn page_composition_emits_common_roles_models_and_configuration_once() {
        let root = PageFixtures::project();
        let files =
            program_files(root.path(), &PageFixtures::services(root.path())).expect("page source");
        let source = PageFixtures::text(&files);
        for declaration in [
            "type PageSetup struct",
            "type PageClient struct",
            "type PageSnapshot struct",
            "func (PageSetup) behaviorConfigure",
            "var behaviorSetups =",
        ] {
            assert_eq!(source.matches(declaration).count(), 1, "{declaration}");
        }
        let (writer, _) = PageFixtures::file(&files, "type WriterService struct");
        let (reader, _) = PageFixtures::file(&files, "type ReaderService struct");
        assert_eq!(writer.parent(), reader.parent());
        for declaration in [
            "type PageSetup struct",
            "type PageClient struct",
            "type PageSnapshot struct",
        ] {
            let (file, _) = PageFixtures::file(&files, declaration);
            assert_eq!(file.parent(), writer.parent(), "{declaration}");
        }
    }

    #[test]
    fn page_composition_keeps_distinct_factories_required_types_and_context_options() {
        let root = PageFixtures::project();
        let files =
            program_files(root.path(), &PageFixtures::services(root.path())).expect("page source");
        let source = PageFixtures::text(&files);
        for class in ["WriterService", "ReaderService"] {
            assert!(source.contains(&format!(
                "func TacCreate{class}() []tac.Member {{\n\tinstance := &{class}{{}}"
            )));
        }
        let (_, registry) = PageFixtures::file(&files, "func registerServices()");
        let (writer, _) = PageFixtures::file(&files, "type WriterService struct");
        let package = writer.parent().expect("package").to_string_lossy();
        assert_eq!(
            registry
                .matches(&format!("\"tachyonservices/{package}\""))
                .count(),
            1
        );
        assert!(registry.contains(&format!(
            "tac.Register(\"writer\", {package}.TacCreateWriterService, []string{{\"*\"}})"
        )));
        assert!(registry.contains(&format!(
            "tac.Register(\"reader\", {package}.TacCreateReaderService, []string{{}})"
        )));
        assert!(source.contains("publisher ApplicationEventPublisher"));
        assert!(source.contains("publisher *ApplicationEventPublisher"));
        for partition in ["public-writer", "public-reader"] {
            assert!(source.contains(&format!(
                "NewBehaviorOptions(\"{partition}\", NewBehaviorSignal())"
            )));
        }
        assert_eq!(source.matches("import \"strings\"").count(), 1);
        assert_eq!(source.matches("import \"strconv\"").count(), 1);
    }

    #[test]
    fn page_composition_keeps_lifecycle_hooks_and_member_exclusions_on_their_root() {
        let root = PageFixtures::project();
        let files =
            program_files(root.path(), &PageFixtures::services(root.path())).expect("page source");
        let (_, writer) = PageFixtures::file(&files, "instance := &WriterService{}");
        let (_, reader) = PageFixtures::file(&files, "instance := &ReaderService{}");
        for (table, start, stop, other) in [
            (writer, "Boot", "Close", "ReaderService"),
            (reader, "Begin", "End", "WriterService"),
        ] {
            assert!(table.contains(&format!("instance.{start}()")));
            assert!(table.contains(&format!("instance.{stop}()")));
            assert!(table.contains("Destroy: true, Hook:"));
            assert!(table.contains("{Name: \"inspect\", Call:"));
            for name in [
                start.to_ascii_lowercase(),
                stop.to_ascii_lowercase(),
                String::from("publisher"),
                String::from("context"),
                String::from("client"),
            ] {
                assert!(!table.contains(&format!("{{Name: \"{name}\"")), "{name}");
            }
            assert!(!table.contains(other));
        }
    }

    #[test]
    fn page_composition_aliases_one_root_without_repeating_its_owner_factory() {
        let root = PageFixtures::project();
        let path = root.path().join("client/services/WriterService.go");
        let services = [
            (
                String::from("writer"),
                String::from("WriterService"),
                path.clone(),
            ),
            (
                String::from("writerTwin"),
                String::from("WriterService"),
                path,
            ),
        ];
        let files = program_files(root.path(), &services).expect("aliased page source");
        let source = PageFixtures::text(&files);
        assert_eq!(source.matches("type WriterService struct").count(), 1);
        assert_eq!(source.matches("func TacCreateWriterService()").count(), 1);
        assert_eq!(source.matches("instance := &WriterService{}").count(), 1);
        let (_, registry) = PageFixtures::file(&files, "func registerServices()");
        assert!(registry.contains("tac.Register(\"writer\","));
        assert!(registry.contains("tac.Register(\"writerTwin\","));
        assert_eq!(registry.matches(".TacCreateWriterService,").count(), 2);
        assert_eq!(source.matches("var behaviorSetups =").count(), 1);
    }

    #[test]
    fn page_composition_preserves_single_root_roles_and_authored_file_lines() {
        let root = PageFixtures::project();
        let services = PageFixtures::services(root.path());
        let files = program_files(root.path(), &services[..1]).expect("single root source");
        let source = PageFixtures::text(&files);
        assert_eq!(source.matches("type WriterService struct").count(), 1);
        assert_eq!(source.matches("type PageSetup struct").count(), 1);
        assert_eq!(source.matches("var behaviorSetups =").count(), 1);
        assert!(!source.contains("type ReaderService struct"));
        for path in [
            "client/services/WriterService.go",
            "client/setups/PageSetup.go",
            "client/remotes/PageClient.go",
            "client/models/PageSnapshot.go",
        ] {
            assert!(source.contains(&format!("//line {path}:1")), "{path}");
        }
    }

    #[test]
    fn page_composition_refuses_equal_root_names_from_distinct_authored_paths() {
        let root = tempfile::tempdir().expect("project");
        let source = "// @Service\ntype CounterService struct { Count int }\n";
        let paths = [
            "client/services/left/CounterService.go",
            "client/services/right/CounterService.go",
        ];
        for path in paths {
            PageFixtures::write(root.path(), path, source);
        }
        let services = paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| {
                (
                    format!("counter{index}"),
                    String::from("CounterService"),
                    root.path().join(path),
                )
            })
            .collect::<Vec<_>>();
        let failure = program_files(root.path(), &services)
            .expect_err("distinct same-name roots must not silently merge");
        let diagnostic = &failure.diagnostics()[0];
        assert!(diagnostic.message.contains("CounterService"), "{failure}");
        for path in paths {
            let span = diagnostic
                .spans
                .iter()
                .find(|span| span.file == path)
                .expect(path);
            assert_eq!(
                span.start,
                u64::try_from(source.find("type CounterService").expect("type")).expect("offset")
            );
        }
    }

    #[test]
    fn page_composition_refuses_a_model_name_that_collides_with_an_owned_root() {
        let root = PageFixtures::project();
        let source = "package main\n\ntype WriterService struct { Value int }\n";
        PageFixtures::write(root.path(), "client/models/WriterService.go", source);
        let failure = program_files(root.path(), &PageFixtures::services(root.path()))
            .expect_err("model/root collision");
        let diagnostic = &failure.diagnostics()[0];
        assert!(diagnostic.message.contains("WriterService"), "{failure}");
        for path in [
            "client/services/WriterService.go",
            "client/models/WriterService.go",
        ] {
            assert!(
                diagnostic.spans.iter().any(|span| span.file == path),
                "{failure}"
            );
        }
    }

    #[test]
    fn page_composition_keeps_helper_refusal_at_the_authored_helper() {
        let root = PageFixtures::project();
        let source = "// @Helper\ntype StepHelper struct{}\n";
        PageFixtures::write(root.path(), "client/helpers/StepHelper.go", source);
        let failure = program_files(root.path(), &PageFixtures::services(root.path()))
            .expect_err("Go Tac Helper is unavailable");
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.to_string(), "TY1308");
        assert!(diagnostic.message.contains("not available"), "{failure}");
        assert_eq!(diagnostic.spans[0].file, "client/helpers/StepHelper.go");
        assert_eq!(
            diagnostic.spans[0].start,
            u64::try_from(source.find("@Helper").expect("marker")).expect("offset")
        );
    }

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
        let Err(failure) = web_go(root.path(), &services, crate::BuildProfile::Release).await
        else {
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
        std::fs::write(&service, "// @Service\ntype BillingService struct {\n\tTotal int\n}\n\n// @OnStart\nfunc (billing *BillingService) Start() {\n\tbilling.Total = 42\n}\n").expect("service");
        std::fs::write(&repository, "// @Store\ntype BillingStore struct{}\n\nfunc (repository *BillingStore) Subtotal() int {\n\treturn 40\n}\n").expect("repository");
        let services = [(
            String::from("billing"),
            String::from("BillingService"),
            service,
        )];
        let built = web_go(root.path(), &services, crate::BuildProfile::Release)
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
                "// @Service\ntype TallyService struct {\n\tCount int\n}\n\n// @OnStart\nfunc (tally *TallyService) Start() {\n\ttally.Count = (&StepHelper{}).Size()\n}\n",
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
        let Err(failure) = web_go(root.path(), &services, crate::BuildProfile::Release).await
        else {
            panic!("Go Tac Helper is unavailable");
        };
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.to_string(), "TY1308");
        assert!(diagnostic.message.contains("not available"), "{failure}");
        let span = &diagnostic.spans[0];
        assert_eq!(span.file, "client/helpers/StepHelper.go");
        let marker = files[1].1.find("@Helper").expect("marker");
        assert_eq!(span.start, u64::try_from(marker).expect("offset"));
    }

    #[tokio::test]
    async fn a_service_tinygo_cannot_build_falls_back_to_the_standard_compiler() {
        let tinygo_missing = std::process::Command::new("tinygo")
            .arg("version")
            .output()
            .is_err();
        if tinygo_missing {
            eprintln!("skipping: tinygo is unavailable");
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let path = root.path().join("client/services/ProfileService.go");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, "import \"runtime\"\n\n// @Service\ntype ProfileService struct{}\n\nfunc (profile *ProfileService) Count() int {\n\tcount, _ := runtime.ThreadCreateProfile(nil)\n\treturn count\n}\n").expect("service");
        let services = [(
            String::from("profile"),
            String::from("ProfileService"),
            path,
        )];
        let built = web_go(root.path(), &services, crate::BuildProfile::Release)
            .await
            .expect("the standard compiler builds what TinyGo cannot");
        assert!(built.wasm.starts_with(b"\0asm"));
        let note = built.note.expect("a release build explains its compiler");
        assert!(
            note.starts_with("Go: the standard Go compiler (because TinyGo could not build"),
            "{note}"
        );
        assert!(String::from_utf8_lossy(&built.glue).contains("globalThis.Go = class"));
        assert!(!String::from_utf8_lossy(&built.glue).contains("wasi_snapshot_preview1"));
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
