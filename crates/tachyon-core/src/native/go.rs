use super::go_compiler_choice::GoCompilerChoice;
use super::go_events::GoEvents;
use super::go_needs::GoNeeds;
use super::go_web_build::GoWebBuild;
use super::host::{native_io, native_tool_failure};
use crate::Failure;
use crate::companion::go::{GoTable, masked_code};
use crate::go_source::GoFile;
use crate::wasm_release_step::WasmReleaseStep;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub(crate) struct GoServices {
    parts: BTreeMap<String, GoFile>,
    owners: BTreeMap<String, PageOwner>,
    registrations: Vec<StagedService>,
    needs: GoNeeds,
}

type PageOwner = (String, GoEvents);

struct StagedService {
    package: String,
    instance: String,
    required: Vec<String>,
    class: Option<String>,
}

impl GoServices {
    const MODULE: &str = "tachyonservices";
    const PAGE_PACKAGE: &str = "page";
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

    fn page(snapshot_root: &Path, services: &[(String, String, PathBuf)]) -> Result<Self, Failure> {
        let mut page = Self {
            parts: BTreeMap::new(),
            owners: BTreeMap::new(),
            registrations: Vec::new(),
            needs: GoNeeds::default(),
        };
        for (instance, class, source) in services {
            let input = ServiceInput {
                snapshot_root,
                class,
                source,
            };
            page.add_owner(instance, &input)?;
        }
        page.needs.resolve()?;
        Ok(page)
    }

    fn add_owner(&mut self, instance: &str, input: &ServiceInput<'_>) -> Result<(), Failure> {
        let (path, parts, required, events) = service_files(input)?;
        let authored = read_service(input.source)?;
        self.needs.add((instance, input.class), &path, &authored)?;
        self.add_class(&path, input.class, events)?;
        for part in parts {
            self.add_part(part)?;
        }
        self.registrations.push(StagedService {
            package: Self::PAGE_PACKAGE.to_owned(),
            instance: instance.to_owned(),
            required,
            class: Some(input.class.to_owned()),
        });
        Ok(())
    }

    fn add_class(&mut self, path: &str, class: &str, events: GoEvents) -> Result<(), Failure> {
        let (kept_class, _) = self
            .owners
            .entry(path.to_owned())
            .or_insert_with(|| (class.to_owned(), events));
        (kept_class.as_str() == class)
            .then_some(())
            .ok_or_else(|| Self::conflicting_source(path))
    }

    fn add_part(&mut self, part: GoFile) -> Result<(), Failure> {
        let path = part.path.clone();
        let same = self
            .parts
            .get(&path)
            .is_none_or(|previous| previous.text == part.text);
        self.parts.entry(path.clone()).or_insert(part);
        same.then_some(())
            .ok_or_else(|| Self::conflicting_source(&path))
    }

    fn conflicting_source(path: &str) -> Failure {
        Failure::one(crate::failure::diagnostic(
            GoServices::FAILURE_CODE,
            "One authored Go path produced conflicting page declarations.",
            None,
            crate::failure::source_span(path, 0, 1),
        ))
    }

    fn validate_names(&self, models: &crate::model_sources::ModelSources) -> Result<(), Failure> {
        let mut names = BTreeMap::new();
        let model_files: Vec<(String, String)> = models
            .project_files()
            .into_iter()
            .map(|(path, source)| (path.to_string_lossy().replace('\\', "/"), source))
            .filter(|(path, _)| !self.parts.contains_key(path))
            .collect();
        let authored = self
            .parts
            .values()
            .map(|part| (part.path.as_str(), part.text.as_str()));
        let modeled = model_files
            .iter()
            .map(|(path, source)| (path.as_str(), source.as_str()));
        for (path, source) in authored.chain(modeled) {
            Self::add_names(&mut names, path, source)?;
        }
        Ok(())
    }

    fn add_names(
        names: &mut BTreeMap<String, (String, usize)>,
        path: &str,
        source: &str,
    ) -> Result<(), Failure> {
        let code = masked_code(source, path)?;
        for declared in crate::go_source::types(&code) {
            let second = (path, declared.start);
            names
                .get(&declared.name)
                .map_or(Ok(()), |(previous_path, previous_start)| {
                    let first = (previous_path.as_str(), *previous_start);
                    Err(Self::duplicate_name(&declared.name, first, second))
                })?;
            names.insert(declared.name, (path.to_owned(), declared.start));
        }
        Ok(())
    }

    fn duplicate_name(name: &str, first: (&str, usize), second: (&str, usize)) -> Failure {
        let (first_path, first_start) = first;
        let (second_path, second_start) = second;
        let message =
            format!("Go page type '{name}' is declared in both {first_path} and {second_path}.");
        let first_span = crate::failure::source_span(first_path, first_start, first_start + 1);
        let second_span = crate::failure::source_span(second_path, second_start, second_start + 1);
        let mut diagnostic =
            crate::failure::diagnostic(GoServices::FAILURE_CODE, message, None, first_span);
        diagnostic.spans.extend(second_span);
        Failure::one(diagnostic)
    }

    fn model_files(models: &crate::model_sources::ModelSources) -> Vec<(PathBuf, String)> {
        let mut files = Vec::new();
        for (name, text) in models.separate_files() {
            let path = Path::new(Self::PAGE_PACKAGE).join(name);
            files.push((path, in_package(&text, Self::PAGE_PACKAGE)));
        }
        files
    }

    fn program(&self) -> String {
        let mut source = String::new();
        for part in self.parts.values() {
            source.push_str(&GoFile::role_separator(&part.name, &part.path));
            source.push_str(&part.text);
        }
        source
    }

    fn files(
        &self,
        snapshot_root: &Path,
        models: &crate::model_sources::ModelSources,
    ) -> Result<Vec<(PathBuf, String)>, Failure> {
        let Some(identity) = self.owners.keys().next() else {
            return Ok(Vec::new());
        };
        let parts = self.lowered_parts(snapshot_root, Path::new(identity))?;
        let mut files = Self::part_files(&parts);
        let tables = self
            .owners
            .iter()
            .enumerate()
            .map(|(index, (path, owner))| {
                Self::owner_file(index, path, owner, (&parts, &self.needs))
            })
            .collect::<Result<Vec<_>, Failure>>()?;
        files.extend(tables);
        files.extend(self.needs.shared_file(Self::PAGE_PACKAGE, Self::MODULE));
        let alias = publisher_alias(Self::PAGE_PACKAGE);
        files.push((Path::new(Self::PAGE_PACKAGE).join("tac.go"), alias));
        files.extend(Self::model_files(models));
        Ok(files)
    }

    fn lowered_parts(&self, snapshot_root: &Path, identity: &Path) -> Result<Vec<GoFile>, Failure> {
        let authored = self.program();
        let metadata = crate::model_sources::ModelSources::client_metadata(snapshot_root, "go")?;
        let lowered = metadata.lowered(identity, &authored, |text| {
            crate::behavior::lower(identity, text, "tac")
                .map_err(|failure| GoFile::locate(&failure, text))
        })?;
        let program = crate::companion::go::without_markers(
            &lowered.unwrap_or(authored),
            &identity.to_string_lossy(),
        )?;
        Ok(GoFile::split(&program))
    }

    fn part_files(parts: &[GoFile]) -> Vec<(PathBuf, String)> {
        parts
            .iter()
            .enumerate()
            .filter(|(_, part)| !part.text.trim().is_empty())
            .map(|(index, part)| {
                let name = format!("part{index}.go");
                (
                    Path::new(Self::PAGE_PACKAGE).join(name),
                    Self::located(part),
                )
            })
            .collect()
    }

    fn located(part: &GoFile) -> String {
        let text = in_package(&part.text, Self::PAGE_PACKAGE);
        if part.path.is_empty() {
            return text;
        }
        format!("//line {}:1\n{text}", part.path)
    }

    fn owner_file(
        index: usize,
        path: &str,
        owner: &PageOwner,
        (parts, needs): (&[GoFile], &GoNeeds),
    ) -> Result<(PathBuf, String), Failure> {
        let part = parts
            .iter()
            .find(|part| part.path == path)
            .ok_or_else(|| Self::conflicting_source(path))?;
        let (class, events) = owner;
        let factory = format!("TacCreate{class}");
        let mut excluded = events.excluded.clone();
        excluded.extend(needs.excluded(class));
        let table = crate::companion::go::owner_table_named(
            &GoTable {
                source: &part.text,
                source_path: path,
                class,
                excluded: &excluded,
                extra: &events.entries,
                needs: &needs.statements(class),
            },
            &factory,
        )?;
        let name = format!("members{index}.go");
        let text = format!(
            "package {}\n\nimport \"{}/tac\"\n\n{table}",
            Self::PAGE_PACKAGE,
            Self::MODULE
        );
        Ok((Path::new(Self::PAGE_PACKAGE).join(name), text))
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
        .map_err(|failure| with_authored_paths(&failure))
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
    let page = GoServices::page(snapshot_root, services)?;
    page.validate_names(&models)?;
    files.extend(page.files(snapshot_root, &models)?);
    files.push((PathBuf::from("services.go"), registry(&page.registrations)));
    Ok(files)
}

struct ServiceInput<'a> {
    snapshot_root: &'a Path,
    class: &'a str,
    source: &'a Path,
}

fn read_service(source: &Path) -> Result<String, Failure> {
    let text = native_io(std::fs::read_to_string(source), source)?;
    Ok(String::from(crate::without_bom(&text)))
}

type ServiceParts = (String, Vec<GoFile>, Vec<String>, GoEvents);

fn service_files(input: &ServiceInput<'_>) -> Result<ServiceParts, Failure> {
    let relative = input
        .source
        .strip_prefix(input.snapshot_root)
        .unwrap_or(input.source);
    let roles = crate::behavior::ServiceRoles::read(relative, &read_service(input.source)?)?;
    let authored = roles.source;
    let portable = relative.to_string_lossy().replace('\\', "/");
    let assembled =
        crate::annotation_modules::assemble(input.snapshot_root, relative, &authored, "tac")?;
    let authored = assembled.unwrap_or(authored);
    let mut events = crate::events::extract(relative, &authored, "tac")?;
    if let Some(module) = events.as_mut() {
        crate::events::describe(module, relative, input.snapshot_root, "client")?;
    }
    let authored = match &events {
        Some(module) => module.source.clone(),
        None => authored,
    };
    let prepared = super::go_events::entries(events.as_ref(), input.class, relative)?;
    let mut parts = GoFile::split(&authored);
    if let Some(entry) = parts.first_mut() {
        entry.path.clone_from(&portable);
        input.class.clone_into(&mut entry.name);
    }
    Ok((portable, parts, roles.required, prepared))
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
    let tac = format!("\t\"{}/tac\"\n", GoServices::MODULE);
    let packages = services
        .iter()
        .map(|service| &service.package)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|package| format!("\t\"{}/{package}\"\n", GoServices::MODULE));
    let imports = std::iter::once(tac).chain(packages).collect::<String>();
    let mut registrations = String::new();
    for service in services {
        let factory = service.class.as_ref().map_or_else(
            || String::from("TacCreate"),
            |class| format!("TacCreate{class}"),
        );
        let _ = writeln!(
            registrations,
            "\ttac.Register({:?}, {}.{factory}, {})",
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

fn with_authored_paths(failure: &Failure) -> Failure {
    let Some(first) = failure.diagnostics().first() else {
        return failure.clone();
    };
    let staged = format!("{}/client/", GoServices::PAGE_PACKAGE);
    let message =
        crate::compiler::PortablePath::tool_output(&first.message).replace(&staged, "client/");
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
    fn a_needed_service_is_assigned_in_the_factory_and_kept_off_the_page() {
        let root = tempfile::tempdir().expect("project");
        let counter = "// @Service\ntype CounterService struct {\n\tCount int\n}\n\nfunc (counter *CounterService) Increment() int {\n\tcounter.Count++\n\treturn counter.Count\n}\n";
        let tally = "// @Service\ntype TallyService struct {\n\t// @Needs\n\tcounter *CounterService\n\tTotal   int\n}\n\nfunc (tally *TallyService) Bump() int {\n\ttally.Total = tally.counter.Increment()\n\treturn tally.Total\n}\n";
        let mut services = Vec::new();
        for (instance, class, source) in [
            ("counter", "CounterService", counter),
            ("tally", "TallyService", tally),
        ] {
            let relative = format!("client/services/{class}.go");
            PageFixtures::write(root.path(), &relative, source);
            services.push((
                String::from(instance),
                String::from(class),
                root.path().join(relative),
            ));
        }
        let files = program_files(root.path(), &services).expect("page source");
        let (_, table) = PageFixtures::file(&files, "func TacCreateTallyService()");
        assert!(table.starts_with("package page\n\nimport \"tachyonservices/tac\"\n\nfunc TacCreateTallyService() []tac.Member {\n\tinstance := &TallyService{}\n\tinstance.counter = tac.Need(\"counter\").(*CounterService)\n\treturn []tac.Member{\n\t\t{Instance: instance},\n"), "{table}");
        assert!(table.contains("{Name: \"total\", Read:"), "{table}");
        assert!(!table.contains("{Name: \"counter\""), "{table}");
        let (_, counter_table) = PageFixtures::file(&files, "func TacCreateCounterService()");
        assert!(
            counter_table.contains("{Name: \"count\", Read:"),
            "{counter_table}"
        );
        assert!(!counter_table.contains("tac.Need("), "{counter_table}");
        let (path, shared) = PageFixtures::file(&files, "tac.Shared(");
        assert_eq!(path, Path::new("page/tac_needs.go"));
        assert!(
            shared.contains("func init() {\n\ttac.Shared(\"counter\")\n}\n"),
            "{shared}"
        );
        let (_, registry) = PageFixtures::file(&files, "func registerServices()");
        assert!(
            registry
                .contains("tac.Register(\"counter\", page.TacCreateCounterService, []string{})")
        );
    }

    #[test]
    fn the_registry_imports_and_registers_every_service() {
        let registry = registry(&[StagedService {
            package: String::from("service0"),
            instance: String::from("counter"),
            required: vec![String::from("label")],
            class: None,
        }]);
        assert!(registry.contains("\t\"tachyonservices/service0\"\n"));
        assert!(
            registry
                .contains("\ttac.Register(\"counter\", service0.TacCreate, []string{\"label\"})\n")
        );
    }
}
