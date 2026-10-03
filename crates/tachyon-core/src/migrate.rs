use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::project::{CompanionKind, NativeCompanion};
use crate::template::{ComponentRegistry, TemplateFrontend};
use html5gum::{Token, Tokenizer};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

mod built_converter;
#[cfg(test)]
mod built_converter_tests;
mod fix;
pub use fix::{MigrationFix, MigrationFixPlan};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    Supported,
    Changed,
    Unsupported,
}

impl MigrationStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Changed => "changed",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationFinding {
    pub source: String,
    pub feature: String,
    pub status: MigrationStatus,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

impl MigrationFinding {
    fn unreadable(relative: &str, failure: &Failure) -> Self {
        let detail = failure.diagnostics().first().map_or_else(
            || failure.to_string(),
            |diagnostic| diagnostic.message.clone(),
        );
        Self::new(
            relative,
            "source.unreadable",
            MigrationStatus::Unsupported,
            &detail,
            Some(
                "Save the file as UTF-8 text, or move it out of the project folders, then run ty migrate check again.",
            ),
        )
    }

    fn new(
        source: &str,
        feature: &str,
        status: MigrationStatus,
        detail: &str,
        action: Option<&str>,
    ) -> Self {
        Self {
            source: String::from(source),
            feature: String::from(feature),
            status,
            detail: String::from(detail),
            action: action.map(String::from),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationReport {
    pub contract_version: u8,
    pub supported: usize,
    pub changed: usize,
    pub unsupported: usize,
    pub findings: Vec<MigrationFinding>,
}

impl MigrationReport {
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.unsupported == 0 && self.changed == 0
    }

    #[must_use]
    pub fn unsupported_failure(&self) -> Option<Failure> {
        if self.unsupported == 0 {
            return None;
        }
        Some(Failure::one(diagnostic(
            1702,
            format!(
                "{} construct(s) have no equivalent in this implementation.",
                self.unsupported
            ),
            Some(String::from(
                "Resolve each unsupported finding, or pass --allow-unsupported \
                 to report without failing.",
            )),
            None,
        )))
    }

    #[must_use]
    pub fn to_text(&self) -> String {
        use std::fmt::Write as _;
        let mut text = String::new();
        for finding in &self.findings {
            let _ = writeln!(
                text,
                "{:<11} {:<28} {}",
                finding.status.label(),
                finding.feature,
                finding.source
            );
            let _ = writeln!(text, "            {}", finding.detail);
            if let Some(action) = &finding.action {
                let _ = writeln!(text, "            action: {action}");
            }
        }
        let _ = writeln!(
            text,
            "\n{} supported, {} changed, {} unsupported",
            self.supported, self.changed, self.unsupported
        );
        text
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MigrationAnalysis;

impl MigrationAnalysis {
    const MAX_SOURCE_BYTES: u64 = 4 * 1_024 * 1_024;
    const MAX_PUBLISHER_SCAN_BYTES: usize = 1024 * 1024;
    const MAX_FINDINGS: usize = 10_000;
    const CONTROL_TAGS: [&'static str; 5] = ["logic", "loop", "if", "else", "for"];
    const IGNORED_DIRECTORIES: [&str; 7] = [
        ".git",
        ".tachyon",
        "dist",
        "dist-bin",
        "node_modules",
        "target",
        "__pycache__",
    ];
    const PROJECT_FOLDERS: [&str; 2] = ["client", "server"];

    fn has_view_bindings(source: &str) -> bool {
        source.contains('{') && source.contains('}')
    }

    fn record_view_controls(relative: &str, present: bool, findings: &mut Vec<MigrationFinding>) {
        if !present {
            return;
        }
        findings.push(MigrationFinding::new(
            relative,
            "view.control_tags",
            MigrationStatus::Supported,
            "Canonical <logic> conditionals and <loop> iteration are validated by the compiler and rendered exclusively by the Tac client. The <if>/<else>/<for> aliases remain supported for compatibility. Yon is REST-only and never renders a view.",
            None,
        ));
    }

    fn record_view_bindings(relative: &str, present: bool, findings: &mut Vec<MigrationFinding>) {
        if !present {
            return;
        }
        findings.push(MigrationFinding::new(
            relative,
            "view.bindings",
            MigrationStatus::Supported,
            "Bounded binding expressions are evaluated with contextual escaping.",
            None,
        ));
    }

    fn record_view_legacy_events(
        relative: &str,
        present: bool,
        findings: &mut Vec<MigrationFinding>,
    ) {
        if !present {
            return;
        }
        findings.push(MigrationFinding::new(
            relative,
            "view.legacy_events",
            MigrationStatus::Changed,
            "Generated data-tac-on-* markers are replaced by on:<event> bindings in the Tac client render plan.",
            Some("Author an on:<event> binding and let the Tac renderer own dispatch."),
        ));
    }

    fn record_view_islands(relative: &str, present: bool, findings: &mut Vec<MigrationFinding>) {
        if !present {
            return;
        }
        findings.push(MigrationFinding::new(
            relative,
            "view.islands",
            MigrationStatus::Changed,
            "Tac has no SSR islands; components are created and mounted entirely in the browser.",
            Some("Use a registered Tac component tag; hydrate= is accepted only as a mount schedule."),
        ));
    }

    fn is_control_token(token: Token) -> bool {
        let Token::StartTag(tag) = token else {
            return false;
        };
        Self::CONTROL_TAGS.contains(&String::from_utf8_lossy(&tag.name).as_ref())
    }

    pub fn check(project_root: impl AsRef<Path>) -> Result<MigrationReport, Failure> {
        let root = project_root.as_ref();
        require_directory(root)?;
        let mut findings = Vec::new();
        let mut signals = ProjectSignals::default();
        scan_project(root, &mut signals, &mut findings)?;
        findings.extend(compile_views(root));
        findings.extend(crate::annotation_layout::CohesionCheck::migration_findings(
            root,
        ));
        findings.extend(crate::annotation_layout::DuplicationCheck::migration_findings(root));
        findings.extend(crate::annotation_layout::DesignCheck::migration_findings(
            root,
        ));
        findings.extend(signals.findings());
        findings.extend(crate::handler::TopicLogMove::migration_findings(root));
        Ok(build_report(findings))
    }
}

fn require_directory(root: &Path) -> Result<(), Failure> {
    let metadata = fs::metadata(root).map_err(|error| {
        migration_failure(&format!(
            "Cannot inspect project '{}': {error}",
            root.display()
        ))
    })?;
    if !metadata.is_dir() {
        return Err(migration_failure(
            "The migration target must be a directory.",
        ));
    }
    Ok(())
}

fn scan_project(
    root: &Path,
    signals: &mut ProjectSignals,
    findings: &mut Vec<MigrationFinding>,
) -> Result<(), Failure> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in sorted_entries(&directory)? {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                migration_failure(&format!("Cannot inspect '{}': {error}", path.display()))
            })?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                if is_project_directory(root, &directory, &name) {
                    pending.push(path);
                }
                continue;
            }
            if metadata.len() > MigrationAnalysis::MAX_SOURCE_BYTES {
                continue;
            }
            let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
            if let Err(unreadable) = classify_file(root, &relative, &name, signals, findings) {
                findings.push(MigrationFinding::unreadable(&relative, &unreadable));
            }
            if findings.len() > MigrationAnalysis::MAX_FINDINGS {
                return Err(migration_failure(
                    "Project exceeds the limit of 10,000 migration findings.",
                ));
            }
        }
    }
    Ok(())
}

fn sorted_entries(directory: &Path) -> Result<Vec<fs::DirEntry>, Failure> {
    let read_failure = |error: std::io::Error| {
        migration_failure(&format!("Cannot read '{}': {error}", directory.display()))
    };
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory).map_err(read_failure)? {
        entries.push(entry.map_err(read_failure)?);
    }
    entries.sort_by_key(fs::DirEntry::file_name);
    Ok(entries)
}

fn is_project_directory(root: &Path, parent: &Path, name: &str) -> bool {
    if parent == root {
        return MigrationAnalysis::PROJECT_FOLDERS.contains(&name);
    }
    !is_ignored_directory(name)
}

fn is_ignored_directory(name: &str) -> bool {
    MigrationAnalysis::IGNORED_DIRECTORIES.contains(&name) || name.starts_with('.')
}

fn build_report(mut findings: Vec<MigrationFinding>) -> MigrationReport {
    findings.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.feature.cmp(&right.feature))
    });
    findings.dedup();
    MigrationReport {
        contract_version: 1,
        supported: count_with_status(&findings, MigrationStatus::Supported),
        changed: count_with_status(&findings, MigrationStatus::Changed),
        unsupported: count_with_status(&findings, MigrationStatus::Unsupported),
        findings,
    }
}

fn count_with_status(findings: &[MigrationFinding], status: MigrationStatus) -> usize {
    findings
        .iter()
        .filter(|finding| finding.status == status)
        .count()
}

#[derive(Default)]
struct ProjectSignals {
    api_paths: BTreeSet<&'static str>,
    telemetry: bool,
    client_module: bool,
}

impl ProjectSignals {
    fn observe(&mut self, source: &str) {
        for token in source.split(Self::is_token_separator) {
            if let Some(endpoint) = Self::api_endpoint(token) {
                self.api_paths.insert(endpoint);
            }
        }
        self.telemetry |= source.contains("telemetry") || source.contains("Telemetry");
    }

    fn is_token_separator(character: char) -> bool {
        character.is_whitespace() || matches!(character, '\'' | '"' | '`' | '<' | '>')
    }

    fn api_endpoint(token: &str) -> Option<&'static str> {
        let path = token.split(['?', '#']).next().unwrap_or_default();
        match path {
            "/openapi.json" => Some("/openapi.json"),
            "/api-docs" | "/api-docs/" | "/api-docs/index.html" => Some("/api-docs"),
            _ => None,
        }
    }

    fn findings(&self) -> Vec<MigrationFinding> {
        let mut findings = Vec::new();
        if self.api_paths.contains("/openapi.json") {
            findings.push(Self::openapi_finding());
        }
        if self.api_paths.contains("/api-docs") {
            findings.push(Self::api_docs_finding());
        }
        if self.telemetry {
            findings.push(Self::telemetry_finding());
        }
        if self.client_module {
            findings.push(Self::navigation_finding());
        }
        findings
    }

    fn openapi_finding() -> MigrationFinding {
        MigrationFinding::new(
            "(project)",
            "server.openapi",
            MigrationStatus::Unsupported,
            "This project names the legacy /openapi.json endpoint, which is not generated. \
             The current /api-docs viewer describes route contracts; it is not OpenAPI-compatible.",
            Some(
                "Generate a specification outside Tachyon, or keep the legacy server \
                 for those endpoints.",
            ),
        )
    }

    fn api_docs_finding() -> MigrationFinding {
        MigrationFinding::new(
            "(project)",
            "server.api_docs",
            MigrationStatus::Supported,
            "The /api-docs viewer and api.json are generated from controller parameter and return model types. \
             This route-contract reference is not OpenAPI-compatible.",
            None,
        )
    }

    fn telemetry_finding() -> MigrationFinding {
        MigrationFinding::new(
            "(project)",
            "server.telemetry",
            MigrationStatus::Unsupported,
            "This project reaches for telemetry, and no OpenTelemetry spans are \
             emitted here.",
            Some("Instrument outside Tachyon if emitted spans are relied upon."),
        )
    }

    fn navigation_finding() -> MigrationFinding {
        MigrationFinding::new(
            "(project)",
            "client.navigation",
            MigrationStatus::Changed,
            "Tac renders each page in the client, while route navigation remains a \
             real cross-document navigation with view transitions and prefetching, \
             so in-memory state does not survive it.",
            Some(
                "Hold state that must outlive a navigation in storage, a shared worker, \
                 or on the server.",
            ),
        )
    }
}

fn configuration_findings(
    root: &Path,
    relative: &str,
    name: &str,
    findings: &mut Vec<MigrationFinding>,
) {
    let is_yon_configuration = is_yon_configuration(name);
    if let Some(finding) = configuration_location_finding(relative, name, is_yon_configuration) {
        findings.push(finding);
    }
    if !is_yon_configuration {
        return;
    }
    if relative != name && relative != format!("server/{name}") {
        return;
    }
    let Some(document) = read_json(root, relative) else {
        return;
    };
    if relative.starts_with("server/") && document.get("yon").is_some() {
        findings.push(yon_key_finding(relative));
    }
    if document.get("tac").is_some() {
        findings.push(tac_key_finding(relative));
    }
}

fn is_yon_configuration(name: &str) -> bool {
    if name == "application.json" {
        return true;
    }
    let is_json = Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    name.starts_with("application-") && is_json
}

fn configuration_location_finding(
    relative: &str,
    name: &str,
    is_yon_configuration: bool,
) -> Option<MigrationFinding> {
    if relative != name {
        return None;
    }
    if is_yon_configuration {
        return Some(MigrationFinding::new(
            relative,
            "config.location",
            MigrationStatus::Changed,
            "Yon's configuration sits beside the server it configures (ADR 0034); a copy at the project root stops the build.",
            Some(&format!(
                "Move {relative} to server/{name}, with the members of its yon section at the top level and no yon key."
            )),
        ));
    }
    if name != "manifest.json" {
        return None;
    }
    Some(MigrationFinding::new(
        relative,
        "config.location",
        MigrationStatus::Changed,
        "The web app manifest sits beside the client it describes (ADR 0034); a copy at the project root stops the build.",
        Some(&format!("Move {relative} to client/manifest.json.")),
    ))
}

fn yon_key_finding(relative: &str) -> MigrationFinding {
    MigrationFinding::new(
        relative,
        "config.yon",
        MigrationStatus::Changed,
        "server/application.json is Yon's namespace itself, so its members are read as yon.* paths without a root key (ADR 0034).",
        Some(
            "Remove the yon root key and keep its members at the top level; paths and YON_ names do not change.",
        ),
    )
}

fn tac_key_finding(relative: &str) -> MigrationFinding {
    MigrationFinding::new(
        relative,
        "config.tac",
        MigrationStatus::Changed,
        "server/application.json and its profiles configure Yon only; Tac's public configuration is the tac member of client/manifest.json, the web app manifest (ADR 0034).",
        Some(
            "Move the tac section into client/manifest.json as its tac member. Everything in the manifest ships to the client.",
        ),
    )
}

fn read_json(root: &Path, relative: &str) -> Option<Value> {
    let source = fs::read_to_string(root.join(relative)).ok()?;
    serde_json::from_str::<Value>(&source).ok()
}

fn read_source(root: &Path, relative: &str, what: &str) -> Result<String, Failure> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| migration_failure(&format!("Cannot read {what} '{relative}': {error}")))
}

fn classify_file(
    root: &Path,
    relative: &str,
    name: &str,
    signals: &mut ProjectSignals,
    findings: &mut Vec<MigrationFinding>,
) -> Result<(), Failure> {
    if let Some(retired) = retired_publisher_finding(root, relative, name) {
        findings.push(retired);
    }
    if let Some(built) = built_converter::BuiltConverter::read_finding(root, relative) {
        findings.push(built);
    }
    configuration_findings(root, relative, name, findings);
    let path = Path::new(relative);
    let language = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if crate::annotation_rules::AnnotationRules::LANGUAGES.contains(&language) || language == "fs" {
        let source = read_source(root, relative, "role source")?;
        if let Err(failure) = crate::annotation_layout::FolderRoleCheck::check(path, &source) {
            for item in failure.diagnostics().iter().filter(|item| {
                item.code.number() == crate::annotation_layout::FolderRoleCheck::CODE
            }) {
                findings.push(MigrationFinding::new(
                    relative,
                    "role.marker",
                    MigrationStatus::Changed,
                    &item.message,
                    item.help.as_deref(),
                ));
            }
        }
    }

    if relative.starts_with("client/clients/") || relative.starts_with("server/clients/") {
        findings.push(MigrationFinding::new(
            relative,
            "role.remotes",
            MigrationStatus::Supported,
            "clients remains a compatibility alias; remotes is the canonical outbound role folder.",
            Some("Optionally run ty migrate fix --write to move files and recognized paths; review the plan first."),
        ));
    }
    let Some(mut finding) = classify_name(relative, name) else {
        return Ok(());
    };
    if name == ".tachyonrc" && declares_interpreters(root, relative) {
        mark_interpreters_removed(&mut finding);
    }
    check_supervised_handler(root, relative, &mut finding)?;
    findings.push(finding);
    observe_file_contents(root, relative, name, signals, findings)
}

fn declares_interpreters(root: &Path, relative: &str) -> bool {
    let Some(document) = read_json(root, relative) else {
        return false;
    };
    let Some(interpreters) = document.get("interpreters") else {
        return false;
    };
    interpreters
        .as_object()
        .is_some_and(|entries| !entries.is_empty())
}

fn mark_interpreters_removed(finding: &mut MigrationFinding) {
    finding.status = MigrationStatus::Changed;
    finding.detail = String::from(
        "The workers object remains supported, but the configured interpreters were removed.",
    );
    finding.action = Some(String::from(
        "Delete every interpreters registration. Keep worker intervals under workers, \
         move workers to a Yon language, and reach other programs through relay(...).",
    ));
}

fn check_supervised_handler(
    root: &Path,
    relative: &str,
    finding: &mut MigrationFinding,
) -> Result<(), Failure> {
    let path = Path::new(relative);
    if finding.feature != "handler.supervised" || !crate::convention::is_yon_language(path) {
        return Ok(());
    }
    let source = read_source(root, relative, "handler")?;
    let Err(failure) = crate::convention::check_handler(path, &source) else {
        return Ok(());
    };
    let Some(diagnostic) = failure.diagnostics().first() else {
        return Ok(());
    };
    finding.status = MigrationStatus::Changed;
    finding.detail.clone_from(&diagnostic.message);
    let mut action = diagnostic.help.clone().unwrap_or_else(|| {
        String::from("Declare a plain type named Handler and put its HTTP methods on that type.")
    });
    if let Some(existing) = finding.action.take() {
        action.push(' ');
        action.push_str(&existing);
    }
    finding.action = Some(action);
    Ok(())
}

fn observe_file_contents(
    root: &Path,
    relative: &str,
    name: &str,
    signals: &mut ProjectSignals,
    findings: &mut Vec<MigrationFinding>,
) -> Result<(), Failure> {
    if name == "tac.html" {
        let source = read_source(root, relative, "view")?;
        signals.observe(&source);
        classify_view(relative, &source, findings);
    }
    if matches!(name, "tac.js" | "tac.ts") {
        signals.observe(&read_source(root, relative, "client module")?);
        signals.client_module = true;
    }
    let is_handler_or_middleware = name.starts_with("yon.")
        || name.starts_with("middleware.")
        || relative.starts_with("server/controllers/");
    if is_handler_or_middleware && crate::convention::is_yon_language(Path::new(name)) {
        signals.observe(&read_source(root, relative, "handler")?);
    }
    Ok(())
}

fn is_other_language_handler(name: &str) -> bool {
    let extension = Path::new(name).extension().and_then(OsStr::to_str);
    name.starts_with("yon.")
        && !crate::convention::is_yon_language(Path::new(name))
        && !matches!(extension, Some("html" | "css"))
}

fn classify_other_language_handler(relative: &str) -> MigrationFinding {
    MigrationFinding::new(
        relative,
        "handler.other_language",
        MigrationStatus::Changed,
        "Arbitrary handler interpreters and executable/shebang handlers were removed. \
         Routes, middleware and workers must use a framework-owned Yon language.",
        Some(
            "Move the endpoint to server/controllers/<Name>Controller in a supported Yon language, \
             mark it @RequestRoute, and call the existing program explicitly through relay(...).",
        ),
    )
}

fn classify_route_schema(relative: &str) -> MigrationFinding {
    MigrationFinding::new(
        relative,
        "server.route_schema",
        MigrationStatus::Changed,
        "OPTIONS.schema.json is no longer read. Controller parameter and return model types define validation and the published API schema (ADR 0048).",
        Some(
            "Declare route parameter markers and model types with the same validation rules, then delete OPTIONS.schema.json. Keep custom validation in the controller when a model type cannot express it.",
        ),
    )
}

fn classify_native_service(relative: &str, language: NativeCompanion) -> MigrationFinding {
    MigrationFinding::new(
        relative,
        "service.native",
        MigrationStatus::Changed,
        &format!(
            "This {} service compiles natively for {} instead of running in the browser.",
            language.label(),
            language.target_names(),
        ),
        Some(
            "Migrate legacy subset-transpiler code to the language's real compiler and native member contract. \
             Select a supported native target; provide <Name>Service.js or .ts for browser behavior and targets \
             this service cannot reach (ADR 0030, ADR 0019).",
        ),
    )
}

fn retired_publisher_finding(root: &Path, relative: &str, name: &str) -> Option<MigrationFinding> {
    let extension = Path::new(name).extension().and_then(OsStr::to_str);
    let is_source_file = matches!(
        extension,
        Some("js" | "ts" | "py" | "php" | "java" | "kt" | "cs" | "rs" | "swift" | "dart")
    );
    if !is_source_file {
        return None;
    }
    let bytes = fs::read(root.join(relative)).ok()?;
    if bytes.len() > MigrationAnalysis::MAX_PUBLISHER_SCAN_BYTES {
        return None;
    }
    let source = std::str::from_utf8(&bytes).ok()?;
    if !uses_event_publisher_annotation(relative, source) {
        return None;
    }
    Some(MigrationFinding::new(
        relative,
        "events.publisher",
        MigrationStatus::Changed,
        "EventPublisher is removed; publication is a call on the injected application event publisher.",
        Some(
            "Declare an uninitialised applicationEventPublisher field and call publishEvent(event, value) where the method used to return (ADR 0031).",
        ),
    ))
}

fn uses_event_publisher_annotation(relative: &str, source: &str) -> bool {
    let Some(extension) = Path::new(relative).extension().and_then(OsStr::to_str) else {
        return false;
    };
    let mask = crate::lexical::code_mask(extension, source);
    for (position, _) in source.match_indices("EventPublisher") {
        let is_code = mask.code.get(position) == Some(&true);
        let text_before = source[..position].trim_end();
        if is_code && text_before.ends_with(['@', '[', '#']) {
            return true;
        }
    }
    false
}

fn service_destination(relative: &str, name: &str) -> String {
    let component_path = relative
        .strip_prefix("client/components/")
        .and_then(|rest| rest.rsplit_once('/'));
    let directory = match component_path {
        Some((directory, _)) => directory,
        None => "",
    };
    let class = service_class_name(directory);
    let extension = name
        .rsplit_once('.')
        .map_or("js", |(_, extension)| extension);
    format!("client/services/{class}Service.{extension}")
}

fn service_class_name(directory: &str) -> String {
    let mut class = String::new();
    for segment in directory.split(['/', '-']) {
        let mut characters = segment.chars();
        let Some(first) = characters.next() else {
            continue;
        };
        class.push(first.to_ascii_uppercase());
        class.push_str(characters.as_str());
    }
    class
}

fn classify_moved_companion(relative: &str, name: &str) -> Option<MigrationFinding> {
    if let Some(finding) = route_role_finding(relative, name) {
        return Some(finding);
    }
    if relative.starts_with("client/services/") {
        return service_finding(relative, name);
    }
    if relative.starts_with("client/components/") && is_component_companion(name) {
        return Some(component_companion_finding(relative, name));
    }
    if relative.starts_with("client/pages/") && crate::project::companion_kind(name).is_some() {
        return Some(page_companion_finding(relative));
    }
    None
}

fn route_role_finding(relative: &str, name: &str) -> Option<MigrationFinding> {
    let parts: Vec<&str> = relative.split('/').collect();
    if parts.len() < 4 || parts[0] != "server" || parts[1] != "routes" {
        return None;
    }
    let folder = parts[parts.len() - 2];
    let is_role_folder = matches!(
        folder,
        "services"
            | "repositories"
            | "stores"
            | "clients"
            | "remotes"
            | "configurations"
            | "setups"
            | "entities"
            | "delegates"
            | "helpers"
            | "mappers"
            | "converters"
    );
    if !is_role_folder {
        return None;
    }
    Some(MigrationFinding::new(
        relative,
        "role.route",
        MigrationStatus::Changed,
        "Role folders are shared per realm. Controllers now live in server/controllers/; server/routes/ is no longer read.",
        Some(&format!(
            "Move it to server/{}/{name} (ADR 0031).",
            crate::annotation_layout::RenamedFolder::new_name("server", folder).unwrap_or(folder)
        )),
    ))
}

fn service_finding(relative: &str, name: &str) -> Option<MigrationFinding> {
    let extension = name.rsplit_once('.').map_or("", |(_, extension)| extension);
    if matches!(extension, "js" | "ts") {
        return Some(MigrationFinding::new(
            relative,
            "service.tac",
            MigrationStatus::Supported,
            "A Tac service is emitted as a browser module and addressed from views by its instance name.",
            None,
        ));
    }
    let language = native_service_language(extension)?;
    Some(classify_native_service(relative, language))
}

fn native_service_language(extension: &str) -> Option<NativeCompanion> {
    match extension {
        "swift" => Some(NativeCompanion::Swift),
        "kt" => Some(NativeCompanion::Kotlin),
        "cs" => Some(NativeCompanion::CSharp),
        "rs" => Some(NativeCompanion::Rust),
        _ => None,
    }
}

fn is_component_companion(name: &str) -> bool {
    if matches!(name, "tachyon-island.js" | "tachyon-island.ts") {
        return true;
    }
    crate::project::companion_kind(name).is_some_and(|kind| !matches!(kind, CompanionKind::Style))
}

fn component_companion_finding(relative: &str, name: &str) -> MigrationFinding {
    let destination = service_destination(relative, name);
    MigrationFinding::new(
        relative,
        "companion.component",
        MigrationStatus::Changed,
        "Components use services; a component directory holds tac.html and tac.css, so this companion no longer belongs beside a view.",
        Some(&format!(
            "Move it to {destination}, annotate the class @Service() with the file's name, and address it from the view as {{<name>.field}} (ADR 0030)."
        )),
    )
}

fn page_companion_finding(relative: &str) -> MigrationFinding {
    MigrationFinding::new(
        relative,
        "companion.page",
        MigrationStatus::Changed,
        "Pages compose components; a page directory holds only tac.html, so this companion no longer belongs beside a page.",
        Some(
            "Move the view into a component under client/components/<name>/ and place <name /> in the page; \
             move behaviour to client/services/<Name>Service.<ext> and a stylesheet beside the component view (ADR 0029, ADR 0030).",
        ),
    )
}

fn classify_name(relative: &str, name: &str) -> Option<MigrationFinding> {
    if let Some(moved) = classify_moved_companion(relative, name) {
        return Some(moved);
    }
    if let Some(finding) = classify_view_or_handler(relative, name) {
        return Some(finding);
    }
    if let Some(finding) = classify_companion(relative, name) {
        return Some(finding);
    }
    if let Some(finding) = classify_server_or_configuration(relative, name) {
        return Some(finding);
    }
    classify_by_location(relative, name)
}

fn classify_view_or_handler(relative: &str, name: &str) -> Option<MigrationFinding> {
    if name == "tac.html" {
        return Some(MigrationFinding::new(
            relative,
            "view.tac",
            MigrationStatus::Supported,
            "Tac view sources are discovered and compiled.",
            None,
        ));
    }
    if name == "yon.html" {
        return Some(MigrationFinding::new(
            relative,
            "view.yon",
            MigrationStatus::Unsupported,
            "Yon is REST-only and does not compile HTML templates.",
            Some(
                "Move application views to client/pages/**/tac.html, or return an explicit text/html response from a RequestRoute controller.",
            ),
        ));
    }
    if name.starts_with("yon.") && crate::convention::is_yon_language(Path::new(name)) {
        return Some(MigrationFinding::new(
            relative,
            "handler.supervised",
            MigrationStatus::Changed,
            "Yon route files are no longer read. Controllers in server/controllers/ declare paths with RequestRoute and HTTP methods with GetRoute, PostRoute and related markers (ADR 0048).",
            Some(
                "Run 'ty migrate fix --write' to move Handler or Controller routes into server/controllers/; resolve any reported manual steps.",
            ),
        ));
    }
    if is_other_language_handler(name) {
        return Some(classify_other_language_handler(relative));
    }
    None
}

fn classify_companion(relative: &str, name: &str) -> Option<MigrationFinding> {
    match name {
        "tac.js" | "tac.ts" => Some(MigrationFinding::new(
            relative,
            "companion.controller",
            MigrationStatus::Changed,
            "Behaviour is a service (ADR 0030); a tac.js or tac.ts is no longer discovered anywhere.",
            Some(
                "Move it to client/services/<Name>Service.js or .ts and annotate the class @Service().",
            ),
        )),
        "tac.py" | "tac.rs" | "tac.kt" | "tac.swift" | "tac.cs" | "tac.dart" => {
            Some(MigrationFinding::new(
                relative,
                "companion.polyglot",
                MigrationStatus::Unsupported,
                "This companion has no supported target at this location. Python is not a service language, and Dart is a browser service language, not a native one.",
                Some(
                    "Behaviour lives in client/services/<Name>Service.<ext>: js, ts or dart for the browser (ADR 0033), swift, kt, cs or rs for a native target (ADR 0030); server logic is a Yon handler.",
                ),
            ))
        }
        "tac.css" | "yon.css" => Some(MigrationFinding::new(
            relative,
            "companion.style",
            MigrationStatus::Supported,
            "Colocated stylesheets are emitted beside the route and linked from \
             the document.",
            None,
        )),
        _ => None,
    }
}

fn classify_server_or_configuration(relative: &str, name: &str) -> Option<MigrationFinding> {
    match name {
        "middleware.js" | "middleware.ts" => Some(MigrationFinding::new(
            relative,
            "server.middleware",
            MigrationStatus::Supported,
            "Root middleware is a class named Middleware, and may be written in any \
             language the project runs. Tachyon calls its before(request) before the \
             route and its after(request) after it, for every request. Returning 204 \
             continues; any other status answers the request.",
            None,
        )),
        "OPTIONS.schema.json" => Some(classify_route_schema(relative)),
        "tachyon.json" => Some(MigrationFinding::new(
            relative,
            "config.application",
            MigrationStatus::Changed,
            "The native application identity lives in client/manifest.json (ADR 0052); tachyon.json stops the build.",
            Some(
                "Run `ty migrate fix --write` to move id and version into tac.application, and name and entry_route into name and start_url.",
            ),
        )),
        "tac.config.js" | "tac.config.mjs" | "tac.config.ts" => Some(MigrationFinding::new(
            relative,
            "config.tac",
            MigrationStatus::Changed,
            "client/manifest.json is the Tac configuration (ADR 0052); a tac.config file stops the build.",
            Some(
                "Run `ty migrate fix --write` to move plain values into the manifest's tac member, then follow its manual steps and delete the file.",
            ),
        )),
        ".tachyonrc" => Some(MigrationFinding::new(
            relative,
            "config.workers",
            MigrationStatus::Supported,
            "Declares bounded worker intervals. The removed interpreters field is classified \
             separately when present.",
            None,
        )),
        _ => None,
    }
}

fn classify_by_location(relative: &str, name: &str) -> Option<MigrationFinding> {
    if relative.starts_with("server/routes/") {
        return Some(MigrationFinding::new(
            relative,
            "server.routes",
            MigrationStatus::Changed,
            "server/routes/ is no longer read; RequestRoute annotations define Yon paths (ADR 0048).",
            Some(
                "Run 'ty migrate fix --write' to move controllers into server/controllers/, then resolve any reported manual steps.",
            ),
        ));
    }
    if relative.starts_with("server/controllers/")
        && crate::convention::is_yon_language(Path::new(name))
    {
        return Some(MigrationFinding::new(
            relative,
            "handler.supervised",
            MigrationStatus::Supported,
            "RequestRoute controllers run under Handler Protocol v1 with their language runtime or compiler.",
            None,
        ));
    }
    if name == "openapi.json" {
        return Some(MigrationFinding::new(
            relative,
            "server.openapi",
            MigrationStatus::Unsupported,
            "The legacy /openapi.json specification is not generated; the current /api-docs route-contract viewer is not OpenAPI-compatible.",
            Some(
                "Generate or serve the OpenAPI specification outside Tachyon. The current API viewer reads controller parameter and return model types.",
            ),
        ));
    }
    if relative.starts_with("server/workers/") {
        return Some(MigrationFinding::new(
            relative,
            "server.worker",
            MigrationStatus::Supported,
            "A worker is a supervised Yon handler invoked on a schedule. Declare its interval \
             under \"workers\" in .tachyonrc; arbitrary interpreter registration is removed.",
            None,
        ));
    }
    let in_layer = ["server/repositories/", "server/stores/", "server/services/"]
        .iter()
        .any(|layer| relative.starts_with(layer));
    if in_layer {
        return classify_layer_dependency(relative);
    }
    None
}

fn classify_layer_dependency(relative: &str) -> Option<MigrationFinding> {
    let extension = Path::new(relative)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    let (status, detail, action) = match extension.as_str() {
        "js" | "ts" | "py" | "php" | "rb" => (
            MigrationStatus::Supported,
            "Same-language imports use the captured server source snapshot under process isolation. \
             Application modules are ordinary language modules; external packages and files outside \
             the snapshot are not automatically bundled.",
            None,
        ),
        "java" | "kt" | "cs" | "rs" | "go" => (
            MigrationStatus::Changed,
            "Compiled Yon adapters compose same-language annotated roles from the captured server \
             source snapshot. Unannotated helper files and external packages are not automatically \
             added to that compilation.",
            Some(
                "Keep one named class per file in server/services/ or server/stores/, add its Service or Store annotation, and inject it into a RequestRoute controller in server/controllers/. Do not assume external-package compilation.",
            ),
        ),
        _ => return None,
    };
    Some(MigrationFinding::new(
        relative,
        "handler.dependency",
        status,
        detail,
        action,
    ))
}

fn classify_view(relative: &str, source: &str, findings: &mut Vec<MigrationFinding>) {
    classify_view_rendering(relative, source, findings);
    classify_view_embedded_content(relative, source, findings);
}

fn classify_view_rendering(relative: &str, source: &str, findings: &mut Vec<MigrationFinding>) {
    let controls = has_control_tag(source);
    let bindings = MigrationAnalysis::has_view_bindings(source);
    let legacy_events = source.contains("data-tac-on-");
    let islands = source.contains("tachyon-island");
    MigrationAnalysis::record_view_controls(relative, controls, findings);
    MigrationAnalysis::record_view_bindings(relative, bindings, findings);
    MigrationAnalysis::record_view_legacy_events(relative, legacy_events, findings);
    MigrationAnalysis::record_view_islands(relative, islands, findings);
}

fn has_control_tag(source: &str) -> bool {
    Tokenizer::new(source).any(|token| token.is_ok_and(MigrationAnalysis::is_control_token))
}

fn classify_view_embedded_content(
    relative: &str,
    source: &str,
    findings: &mut Vec<MigrationFinding>,
) {
    if source.contains("<script") {
        findings.push(MigrationFinding::new(
            relative,
            "view.inline_script",
            MigrationStatus::Unsupported,
            "Inline scripts are not emitted into generated output.",
            Some(
                "Move behavior into a Tac companion module; keep only bounded literal page-state declarations inline.",
            ),
        ));
    }
    if source.contains("<iframe") {
        findings.push(MigrationFinding::new(
            relative,
            "view.remote_frame",
            MigrationStatus::Changed,
            "An iframe remains browser content inside the platform web view on native targets. \
             Subframes and remote content never receive the native bridge, and host policies may block frames.",
            Some(
                "Review each target's content and navigation policies before relying on embedded frames. Keep native operations in a trusted page companion, not an iframe.",
            ),
        ));
    }
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn migration_failure(message: &str) -> Failure {
    Failure::one(diagnostic(
        1701,
        message,
        Some(String::from(
            "Point `ty migrate check` at a readable Tachyon project directory.",
        )),
        source_span(".", 0, 0),
    ))
}

fn compile_views(root: &Path) -> Vec<MigrationFinding> {
    let (names, mut findings) = match ComponentRegistry::discover(root) {
        Ok(registry) => (registry.names(), Vec::new()),
        Err(failure) => (
            BTreeSet::new(),
            failure.diagnostics().iter().map(compiler_finding).collect(),
        ),
    };
    for path in view_sources(root) {
        let relative = portable_path(path.strip_prefix(root).unwrap_or(&path));
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let diagnostics = match TemplateFrontend::compile(&source, &relative, &names) {
            Ok(program) => crate::template::AccessibilityCheck::program_diagnostics(&program),
            Err(failure) => failure.diagnostics().to_vec(),
        };
        findings.extend(diagnostics.iter().map(compiler_finding));
    }
    findings.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.detail.cmp(&right.detail))
    });
    findings.dedup();
    findings
}

fn view_sources(root: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                if !is_ignored_directory(&name) {
                    pending.push(entry.path());
                }
                continue;
            }
            if name == "tac.html" {
                sources.push(entry.path());
            }
        }
    }
    sources.sort();
    sources
}

fn compiler_finding(diagnostic: &tachyon_diagnostics::Diagnostic) -> MigrationFinding {
    let source = match diagnostic.spans.first() {
        Some(span) => span.file.clone(),
        None => String::from("(project)"),
    };
    MigrationFinding {
        source,
        feature: format!("view.compile.{}", diagnostic.code),
        status: MigrationStatus::Unsupported,
        detail: diagnostic.message.clone(),
        action: diagnostic.help.clone(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{MigrationAnalysis, MigrationFinding, MigrationReport, MigrationStatus};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn project() -> TempDir {
        tempfile::tempdir().expect("project")
    }

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        fs::write(path, contents).expect("source");
    }

    fn check(root: &Path) -> MigrationReport {
        MigrationAnalysis::check(root).expect("report")
    }

    #[test]
    fn old_topic_logs_in_the_generated_folder_are_reported() {
        let root = tempfile::tempdir().expect("project");
        write(root.path(), ".tachyon/topics/order-placed.jsonl", "{}\n");
        let report = check(root.path());
        let finding = with_source(&report, ".tachyon/topics");
        assert_eq!(finding.status, MigrationStatus::Changed);
        assert!(finding.detail.contains("data/topics"));
    }

    #[test]
    fn a_file_that_is_not_utf8_text_is_reported_and_the_check_goes_on() {
        let root = project();
        let broken = root.path().join("client/services/BrokenService.js");
        fs::create_dir_all(broken.parent().expect("parent")).expect("directory");
        fs::write(&broken, [0xff, 0xfe, 0x00]).expect("bytes");
        write(root.path(), "client/pages/tac.html", "<p>kept</p>\n");
        let report = check(root.path());
        let finding = with_feature(&report, "source.unreadable");
        assert_eq!(finding.source, "client/services/BrokenService.js");
        assert_eq!(finding.status, MigrationStatus::Unsupported);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.source == "client/pages/tac.html")
        );
    }

    fn with_source<'a>(report: &'a MigrationReport, source: &str) -> &'a MigrationFinding {
        report
            .findings
            .iter()
            .find(|finding| finding.source == source)
            .unwrap_or_else(|| panic!("{source} was not classified"))
    }

    fn with_source_ending<'a>(report: &'a MigrationReport, suffix: &str) -> &'a MigrationFinding {
        report
            .findings
            .iter()
            .find(|finding| finding.source.ends_with(suffix))
            .unwrap_or_else(|| panic!("{suffix} was not classified"))
    }

    fn with_feature<'a>(report: &'a MigrationReport, feature: &str) -> &'a MigrationFinding {
        report
            .findings
            .iter()
            .find(|finding| finding.feature == feature)
            .unwrap_or_else(|| panic!("{feature} missing from {}", report.to_text()))
    }

    fn has_feature(report: &MigrationReport, feature: &str) -> bool {
        report
            .findings
            .iter()
            .any(|finding| finding.feature == feature)
    }

    fn action_contains(finding: &MigrationFinding, text: &str) -> bool {
        finding
            .action
            .as_deref()
            .is_some_and(|action| action.contains(text))
    }

    fn handler_source(extension: &str) -> String {
        match extension {
            "php" => String::from("<?php\nclass Handler {}"),
            "rs" => String::from("struct Handler;"),
            "go" => String::from("type Handler struct{}"),
            "py" => String::from("class Handler:\n    pass\n"),
            "rb" => String::from("class Handler\nend\n"),
            "lua" => String::from("Handler = {}\n"),
            _ => String::from("class Handler {}"),
        }
    }

    fn module_source(extension: &str, role: &str) -> String {
        let name = format!("Order{role}");
        match extension {
            "php" => format!("<?php\nclass {name} {{}}"),
            "rs" => format!("struct {name};"),
            "go" => format!("type {name} struct{{}}"),
            "py" => format!("class {name}:\n    pass\n"),
            "rb" => format!("class {name}\nend\n"),
            "lua" => format!("{name} = {{}}\n"),
            _ => format!("class {name} {{}}"),
        }
    }

    fn write_layer_modules(root: &Path, extension: &str) {
        for (directory, layer) in [("services", "Service"), ("repositories", "Store")] {
            write(
                root,
                &format!("server/{directory}/orders.{extension}"),
                &module_source(extension, layer),
            );
        }
    }

    #[test]
    fn a_supported_project_reports_clean() {
        let root = project();
        write(
            root.path(),
            "client/pages/tac.html",
            "<main aria-label=\"Home\"><h1>Home</h1></main>",
        );
        write(
            root.path(),
            "server/controllers/HomeController.js",
            "@RequestRoute\nexport class HomeController {\n/** @returns {Object} */\n@GetRoute\nget() { return {} }\n}\n",
        );
        let report = check(root.path());
        assert!(report.is_clean(), "{}", report.to_text());
        assert_eq!(report.unsupported, 0);
        assert!(report.supported >= 2);
    }

    #[test]
    fn only_client_server_and_root_files_are_checked() {
        let root = project();
        write(root.path(), "tachyon.json", "{}");
        write(
            root.path(),
            "content/examples/server/routes/orders/yon.js",
            &handler_source("js"),
        );
        write(root.path(), "docs/tac.config.js", "export default {}");
        let report = check(root.path());
        let sources: Vec<&str> = report
            .findings
            .iter()
            .map(|finding| finding.source.as_str())
            .collect();
        assert_eq!(sources, ["tachyon.json"], "{}", report.to_text());
    }

    #[test]
    fn all_owned_yon_route_languages_require_annotation_migration() {
        for extension in crate::convention::YonConvention::EXTENSIONS {
            let root = project();
            let source = format!("server/routes/orders/yon.{extension}");
            write(root.path(), &source, &handler_source(extension));
            let report = check(root.path());
            let handler = with_source(&report, &source);
            assert_eq!(handler.feature, "handler.supervised", "{extension}");
            assert_eq!(handler.status, MigrationStatus::Changed, "{extension}");
            assert!(handler.detail.contains("RequestRoute"));
            assert!(
                action_contains(handler, "ty migrate fix --write"),
                "{handler:?}"
            );
        }
    }

    #[test]
    fn interpreted_application_modules_remain_importable_from_the_captured_project() {
        for extension in ["js", "ts", "py", "php", "rb"] {
            let root = project();
            write_layer_modules(root.path(), extension);
            let report = check(root.path());
            assert_eq!(report.findings.len(), 3);
            let marker = report
                .findings
                .iter()
                .find(|finding| finding.feature == "role.marker")
                .expect("unregistered service migration");
            assert_eq!(marker.status, MigrationStatus::Changed);
            assert!(marker.source.starts_with("server/services/"));
            for finding in report
                .findings
                .iter()
                .filter(|finding| finding.feature != "role.marker")
            {
                assert_eq!(finding.status, MigrationStatus::Supported, "{finding:?}");
                assert!(finding.detail.contains("snapshot"));
                assert!(finding.detail.contains("imports"));
                assert!(!finding.detail.contains("does not import"));
            }
        }
    }

    #[test]
    fn compiled_application_files_use_annotated_role_composition_not_colocation() {
        for extension in ["java", "kt", "cs", "rs", "go"] {
            let root = project();
            write_layer_modules(root.path(), extension);
            let report = check(root.path());
            let dependencies: Vec<&MigrationFinding> = report
                .findings
                .iter()
                .filter(|finding| finding.feature == "handler.dependency")
                .collect();
            assert_eq!(dependencies.len(), 2, "{extension}: {}", report.to_text());
            for finding in dependencies {
                assert_eq!(finding.status, MigrationStatus::Changed, "{finding:?}");
                assert!(finding.detail.contains("annotated roles"), "{finding:?}");
                assert!(action_contains(finding, "one named class per file"));
                assert!(action_contains(finding, "server/controllers/"));
                assert!(!action_contains(finding, "yon."));
            }
        }
    }

    #[test]
    fn role_files_beside_a_route_move_to_their_realm_shared_folder() {
        let folders = [
            "services",
            "repositories",
            "clients",
            "configurations",
            "entities",
            "delegates",
        ];
        let routes = [
            "server/routes/orders",
            "server/routes/orders/_id",
            "server/routes",
        ];
        for folder in folders {
            for route in routes {
                let root = project();
                let source = format!("{route}/{folder}/Money.js");
                write(root.path(), &source, "export class Money {}");
                let report = check(root.path());
                let finding = with_source(&report, &source);
                assert_eq!(finding.feature, "role.route", "{source}");
                assert_eq!(finding.status, MigrationStatus::Changed, "{source}");
                assert!(finding.detail.contains("shared per realm"), "{finding:?}");
                let target = crate::annotation_layout::RenamedFolder::new_name("server", folder)
                    .unwrap_or(folder);
                let expected_action = format!("Move it to server/{target}/Money.js (ADR 0031).");
                assert_eq!(
                    finding.action.as_deref(),
                    Some(expected_action.as_str()),
                    "{finding:?}"
                );
                assert!(!report.is_clean(), "{source}");
            }
        }
    }

    #[test]
    fn old_route_controllers_move_but_realm_role_folders_stay_in_place() {
        let root = project();
        write(
            root.path(),
            "server/routes/orders/controllers/OrdersController.js",
            "export class OrdersController {}",
        );
        write(
            root.path(),
            "server/helpers/AuditHelper.js",
            "export class AuditHelper {}",
        );
        let report = check(root.path());
        assert!(
            has_feature(&report, "server.routes"),
            "{:?}",
            report.findings
        );
        assert!(!has_feature(&report, "role.route"), "{:?}", report.findings);
    }

    #[test]
    fn layer_modules_are_plain_application_modules() {
        let root = project();
        write(
            root.path(),
            "server/services/orders.js",
            "export class OrdersService {}",
        );
        let report = check(root.path());
        let finding = report.findings.first().expect("layer finding");
        assert_eq!(finding.status, MigrationStatus::Supported);
        assert!(finding.action.is_none());
    }

    #[test]
    fn a_view_the_compiler_refuses_is_reported_before_the_build_fails() {
        let root = project();
        write(
            root.path(),
            "client/pages/tac.html",
            "<main aria-label=\"T\"><p>{bad syntax}</p></main>",
        );
        let report = check(root.path());
        assert!(!report.is_clean(), "{}", report.to_text());
        let finding = report
            .findings
            .iter()
            .find(|finding| finding.feature.starts_with("view.compile."))
            .unwrap_or_else(|| panic!("no compile finding in {}", report.to_text()));
        assert_eq!(finding.source, "client/pages/tac.html");
        assert!(finding.detail.contains("syntax"), "{finding:?}");

        let clean = project();
        write(
            clean.path(),
            "client/pages/tac.html",
            "<main aria-label=\"T\"><h1>T</h1></main>",
        );
        let report = check(clean.path());
        let has_compile_finding = report
            .findings
            .iter()
            .any(|finding| finding.feature.starts_with("view.compile."));
        assert!(!has_compile_finding, "{}", report.to_text());
    }

    #[test]
    fn retained_worker_configuration_is_not_reported_as_a_blocker() {
        let root = project();
        write(
            root.path(),
            "client/pages/tac.html",
            "<main aria-label=\"Home\"><h1>Home</h1></main>",
        );
        write(root.path(), "middleware.js", "export default {}");
        write(root.path(), "server/workers/job.js", "export default {}");
        write(
            root.path(),
            ".tachyonrc",
            r#"{"workers":{"server/workers/job.js":{"every_seconds":30}}}"#,
        );
        let report = check(root.path());
        for feature in ["server.middleware", "server.worker"] {
            let found = with_feature(&report, feature);
            assert_eq!(
                found.status,
                MigrationStatus::Supported,
                "{feature}: {}",
                report.to_text()
            );
        }
    }

    #[test]
    fn a_removed_handler_language_is_reported_with_relay_migration() {
        let root = project();
        write(root.path(), "client/pages/tac.html", "<main>x</main>");
        write(root.path(), "server/routes/b/yon.swift", "// handler");
        let report = check(root.path());
        let finding = with_feature(&report, "handler.other_language");
        assert_eq!(finding.status, MigrationStatus::Changed);
        assert!(action_contains(finding, "relay(...)"), "{finding:?}");
    }

    #[test]
    fn a_tac_section_in_application_json_moves_to_the_manifest() {
        let root = project();
        write(root.path(), "client/pages/tac.html", "<main>x</main>");
        write(
            root.path(),
            "server/application.json",
            r#"{"tac":{"api":{"url":"/api"}},"yon":{}}"#,
        );
        write(
            root.path(),
            "server/application-production.json",
            r#"{"yon":{}}"#,
        );
        write(root.path(), "manifest.json", r#"{"name":"Catalog"}"#);
        let report = check(root.path());

        let tac_findings: Vec<&MigrationFinding> = report
            .findings
            .iter()
            .filter(|finding| finding.feature == "config.tac")
            .collect();
        assert_eq!(tac_findings.len(), 1, "{tac_findings:?}");
        assert_eq!(tac_findings[0].source, "server/application.json");
        assert_eq!(tac_findings[0].status, MigrationStatus::Changed);
        assert!(
            action_contains(tac_findings[0], "client/manifest.json"),
            "{tac_findings:?}"
        );

        let moved = with_feature(&report, "config.location");
        assert_eq!(moved.source, "manifest.json");
        assert_eq!(
            moved.action.as_deref(),
            Some("Move manifest.json to client/manifest.json.")
        );

        let has_yon_finding = report.findings.iter().any(|finding| {
            finding.feature == "config.yon" && finding.source == "server/application.json"
        });
        assert!(has_yon_finding, "{:?}", report.findings);
    }

    #[test]
    fn removed_interpreter_registration_keeps_workers_and_requires_relay() {
        let root = project();
        write(root.path(), "client/pages/tac.html", "<main>x</main>");
        write(
            root.path(),
            ".tachyonrc",
            r#"{"interpreters":{"rb":["ruby"]},"workers":{"server/workers/beat.js":{"every_seconds":30}}}"#,
        );
        let report = check(root.path());
        let finding = with_source(&report, ".tachyonrc");
        assert_eq!(finding.status, MigrationStatus::Changed);
        assert_eq!(finding.feature, "config.workers");
        assert!(finding.detail.contains("interpreters"), "{finding:?}");
        assert!(action_contains(finding, "relay(...)"), "{finding:?}");
    }

    #[test]
    fn a_misnamed_yon_source_is_reported_as_a_required_migration() {
        let root = project();
        write(root.path(), "client/pages/tac.html", "<main>x</main>");
        write(
            root.path(),
            "server/routes/yon.py",
            "class LegacyHandler: pass",
        );
        let report = check(root.path());
        let finding = with_source(&report, "server/routes/yon.py");
        assert_eq!(finding.status, MigrationStatus::Changed);
        assert!(action_contains(finding, "Handler"), "{finding:?}");
    }

    #[test]
    fn unmarked_nested_roles_are_reported_without_blind_registration_edits() {
        let root = tempfile::tempdir().expect("project");
        let path = root.path().join("client/services/nested/MoneyService.js");
        fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        fs::write(&path, "export default class MoneyService {}\n").expect("source");
        let report = MigrationAnalysis::check(root.path()).expect("migration report");
        let item = report
            .findings
            .iter()
            .find(|item| item.feature == "role.marker")
            .expect("explicit registration finding");
        assert_eq!(item.status, MigrationStatus::Changed);
        assert_eq!(item.source, "client/services/nested/MoneyService.js");
        assert!(item.detail.contains("Service"));
        assert_eq!(
            fs::read_to_string(path).expect("unchanged source"),
            "export default class MoneyService {}\n"
        );
    }

    #[test]
    fn surfaces_the_legacy_server_provided_are_reported_only_where_reached() {
        let quiet = project();
        write(quiet.path(), "client/pages/tac.html", "<main>x</main>");
        write(
            quiet.path(),
            "server/controllers/HomeController.js",
            "@RequestRoute\nexport class HomeController {\n/** @returns {Object} */\n@GetRoute\nget() { return {} }\n}\n",
        );
        let report = check(quiet.path());
        assert!(report.is_clean(), "{}", report.to_text());

        let loud = project();
        write(loud.path(), "client/pages/tac.html", "<main>x</main>");
        write(
            loud.path(),
            "server/routes/yon.js",
            "import { telemetry } from 'x'\nexport class Handler {}\nfetch('/openapi.json')",
        );
        let report = check(loud.path());
        let project_features: Vec<&str> = report
            .findings
            .iter()
            .filter(|finding| finding.source == "(project)")
            .map(|finding| finding.feature.as_str())
            .collect();
        assert!(
            project_features.contains(&"server.openapi"),
            "{project_features:?}"
        );
        assert!(
            project_features.contains(&"server.telemetry"),
            "{project_features:?}"
        );
    }

    #[test]
    fn generated_api_viewer_references_are_not_legacy_openapi_dependencies() {
        for endpoint in ["/api-docs", "/api-docs/", "/api-docs/index.html?mode=read"] {
            let root = project();
            write(
                root.path(),
                "client/pages/tac.html",
                &format!("<a href=\"{endpoint}\">API reference</a>"),
            );
            let report = check(root.path());
            let viewer = with_feature(&report, "server.api_docs");
            assert_eq!(viewer.status, MigrationStatus::Supported);
            assert!(viewer.detail.contains("model types"));
            assert!(viewer.detail.contains("not OpenAPI"));
            assert!(!has_feature(&report, "server.openapi"));
        }
    }

    #[test]
    fn api_path_detection_distinguishes_endpoints_from_unrelated_names() {
        let root = project();
        write(
            root.path(),
            "client/pages/tac.html",
            "<a href=\"/api-docs-theme.css\">Theme</a><a href=\"/openapi.json.backup\">Backup</a>",
        );
        write(root.path(), "client/shared/api-docs-theme.css", "a {}");
        let report = check(root.path());
        assert!(!has_feature(&report, "server.openapi"));
        assert!(!has_feature(&report, "server.api_docs"));

        write(
            root.path(),
            "client/pages/tac.html",
            "<a href=\"/api-docs/\">API</a><a href=\"/openapi.json?download=1\">OpenAPI</a>",
        );
        let report = check(root.path());
        let legacy = with_feature(&report, "server.openapi");
        assert_eq!(legacy.status, MigrationStatus::Unsupported);
        assert!(legacy.detail.contains("not OpenAPI"));
        assert!(has_feature(&report, "server.api_docs"));
    }

    fn write_project_with_every_construct(root: &Path) {
        write(root, "client/pages/tac.html", "<app-view />");
        write(root, "client/pages/tac.css", "h1 {}");
        write(
            root,
            "client/components/app/view/tac.html",
            "<main>x</main>",
        );
        write(
            root,
            "client/components/app/view/tac.js",
            "export default {}",
        );
        write(root, "client/components/app/view/tac.rs", "fn main() {}");
        write(root, "client/shared/scripts/tac.js", "export default {}");
        write(root, "client/shared/tac.py", "x = 1");
        write(root, "server/routes/yon.rs", "fn main() {}");
        write(root, "middleware.js", "export default {}");
        write(root, "server/workers/job.js", "export {}");
        write(root, ".tachyonrc", "{}");
    }

    #[test]
    fn unsupported_and_changed_constructs_are_named_with_an_action() {
        let root = project();
        write_project_with_every_construct(root.path());

        let report = check(root.path());
        assert!(!report.is_clean());
        for expected in [
            "companion.page",
            "companion.component",
            "companion.controller",
            "companion.polyglot",
            "handler.supervised",
            "server.middleware",
            "server.worker",
            "config.workers",
        ] {
            assert!(has_feature(&report, expected), "missing {expected}");
        }
        for finding in &report.findings {
            if finding.status == MigrationStatus::Supported {
                continue;
            }
            assert!(
                finding.action.is_some(),
                "{} has no action",
                finding.feature
            );
        }
    }

    fn control_report(source: &str) -> MigrationReport {
        let root = project();
        write(root.path(), "client/pages/tac.html", source);
        check(root.path())
    }

    #[test]
    fn canonical_logic_controls_are_reported_as_supported() {
        let report = control_report(
            "<logic :if=\"ready\"><p>ready</p></logic><logic :else-if=\"waiting\"><p>waiting</p></logic><logic else><p>fallback</p></logic>",
        );
        let finding = with_feature(&report, "view.control_tags");
        assert_eq!(finding.status, MigrationStatus::Supported);
        assert!(finding.detail.contains("Canonical <logic>"));
        assert!(finding.detail.contains("compatibility"));
        assert_eq!(finding.source, "client/pages/tac.html");
    }

    #[test]
    fn canonical_loop_controls_are_reported_as_supported() {
        let report = control_report("<loop :for=\"const item of items\"><p>{item}</p></loop>");
        assert_eq!(
            with_feature(&report, "view.control_tags").status,
            MigrationStatus::Supported
        );
    }

    #[test]
    fn compatible_conditional_aliases_remain_reported_as_supported() {
        let report =
            control_report("<if :when=\"ready\"><p>ready</p></if><else><p>fallback</p></else>");
        assert_eq!(
            with_feature(&report, "view.control_tags").status,
            MigrationStatus::Supported
        );
    }

    #[test]
    fn compatible_iteration_alias_remains_reported_as_supported() {
        let report = control_report("<for :each=\"const item of items\"><p>{item}</p></for>");
        assert_eq!(
            with_feature(&report, "view.control_tags").status,
            MigrationStatus::Supported
        );
    }

    #[test]
    fn logic_named_custom_elements_are_not_control_tags() {
        let report = control_report("<logic-panel></logic-panel>");
        assert!(!has_feature(&report, "view.control_tags"));
    }

    #[test]
    fn conditional_text_inside_a_comment_is_not_a_control_tag() {
        let report = control_report("<main><!-- <logic else>literal</logic> --><p>text</p></main>");
        assert!(!has_feature(&report, "view.control_tags"));
    }

    #[test]
    fn escaped_conditional_text_is_not_a_control_tag() {
        let report = control_report("<main>&lt;logic else&gt;literal&lt;/logic&gt;</main>");
        assert!(!has_feature(&report, "view.control_tags"));
    }

    #[test]
    fn iframe_is_not_the_conditional_alias() {
        let report =
            control_report("<iframe src=\"https://example.test\" title=\"External\"></iframe>");
        assert!(!has_feature(&report, "view.control_tags"));
        assert!(has_feature(&report, "view.remote_frame"));
    }

    #[test]
    fn view_constructs_are_classified_from_source() {
        let root = project();
        write(
            root.path(),
            "client/pages/tac.html",
            r#"<main><logic :if="ok"><p>{title}</p></logic>
               <button data-tac-on-click="go">Go</button>
               <script>alert(1)</script>
               <iframe src="https://example.test"></iframe></main>"#,
        );
        let report = check(root.path());
        let status_of = |feature: &str| with_feature(&report, feature).status;
        assert_eq!(status_of("view.control_tags"), MigrationStatus::Supported);
        assert_eq!(status_of("view.bindings"), MigrationStatus::Supported);
        assert_eq!(status_of("view.legacy_events"), MigrationStatus::Changed);
        assert_eq!(
            status_of("view.inline_script"),
            MigrationStatus::Unsupported
        );
        assert_eq!(status_of("view.remote_frame"), MigrationStatus::Changed);
    }

    #[test]
    fn view_migration_describes_client_rendering_and_host_frame_restrictions() {
        let root = project();
        write(
            root.path(),
            "client/pages/tac.html",
            "<main><logic :if=\"true\">Ready</logic><iframe src=\"https://example.test\" title=\"External\"></iframe></main>",
        );
        let report = check(root.path());
        let controls = with_feature(&report, "view.control_tags");
        assert!(controls.detail.contains("client"));
        assert!(controls.detail.contains("REST-only"));
        assert!(!controls.detail.contains("Yon server runtime"));
        let frame = with_feature(&report, "view.remote_frame");
        assert!(frame.detail.contains("platform web view"));
        assert!(frame.detail.contains("native bridge"));
        assert!(action_contains(frame, "content") && action_contains(frame, "navigation"));
        assert!(!frame.detail.contains("WebSurface"));
    }

    #[test]
    fn generated_and_vendored_directories_are_never_analyzed() {
        let root = project();
        write(root.path(), "client/pages/tac.html", "<main>x</main>");
        write(root.path(), "node_modules/pkg/tac.rs", "fn main() {}");
        write(root.path(), "dist/web/tac.js", "export {}");
        write(root.path(), "target/debug/middleware.js", "export {}");
        let report = check(root.path());
        assert!(report.is_clean(), "{}", report.to_text());
    }

    #[test]
    fn page_languages_without_a_native_target_remain_unsupported() {
        for companion in ["tac.py", "tac.dart"] {
            let root = project();
            write(root.path(), "client/pages/tac.html", "<main>x</main>");
            write(root.path(), &format!("client/pages/{companion}"), "x");
            let report = check(root.path());
            let finding = with_source_ending(&report, companion);
            assert_eq!(
                finding.status,
                MigrationStatus::Unsupported,
                "{companion} is reported as {:?} but fails the build",
                finding.status
            );
            assert!(finding.action.is_some(), "{companion} carries no action");
        }
    }

    #[test]
    fn page_companions_are_moved_into_components() {
        let root = project();
        write(root.path(), "client/pages/tac.html", "<app-panel />");
        write(
            root.path(),
            "client/pages/tac.js",
            "export default class {}",
        );
        write(root.path(), "client/pages/tac.css", "h1 {}");
        write(
            root.path(),
            "client/pages/tac.swift",
            "final class Companion {}",
        );
        let report = check(root.path());
        for name in ["tac.js", "tac.css", "tac.swift"] {
            let finding = with_source(&report, &format!("client/pages/{name}"));
            assert_eq!(finding.feature, "companion.page", "{name}");
            assert_eq!(finding.status, MigrationStatus::Changed, "{name}");
            let action = finding.action.as_deref().expect("move guidance");
            assert!(
                action.contains("client/components/<name>/")
                    && action.contains("client/services/<Name>Service.<ext>"),
                "{finding:?}"
            );
        }
    }

    #[test]
    fn companions_beside_a_component_are_moved_to_a_named_service() {
        for (companion, extension) in [
            ("tac.js", "js"),
            ("tac.ts", "ts"),
            ("tac.rs", "rs"),
            ("tac.swift", "swift"),
            ("tac.kt", "kt"),
            ("tac.cs", "cs"),
            ("tachyon-island.js", "js"),
        ] {
            let root = project();
            let source = format!("client/components/app/panel/{companion}");
            write(root.path(), "client/pages/tac.html", "<app-panel />");
            write(
                root.path(),
                "client/components/app/panel/tac.html",
                "<main>x</main>",
            );
            write(root.path(), &source, "legacy source");
            let report = check(root.path());
            let finding = with_source(&report, &source);
            assert_eq!(finding.feature, "companion.component", "{companion}");
            assert_eq!(finding.status, MigrationStatus::Changed, "{companion}");
            let action = finding.action.as_deref().expect("migration");
            let destination = format!("client/services/AppPanelService.{extension}");
            assert!(action.contains(&destination), "{finding:?}");
            assert!(
                action.contains("@Service()") && action.contains("ADR 0030"),
                "{finding:?}"
            );
        }
    }

    #[test]
    fn a_hyphenated_component_directory_names_a_pascal_case_service() {
        let root = project();
        write(root.path(), "client/pages/tac.html", "<user-list-card />");
        write(
            root.path(),
            "client/components/user-list/card/tac.html",
            "<main>x</main>",
        );
        write(
            root.path(),
            "client/components/user-list/card/tac.ts",
            "export default class {}",
        );
        let report = check(root.path());
        let finding = with_source_ending(&report, "tac.ts");
        assert!(
            action_contains(finding, "client/services/UserListCardService.ts"),
            "{finding:?}"
        );
    }

    fn write_services_project(root: &Path) {
        write(root, "client/pages/tac.html", "<app-panel />");
        write(
            root,
            "client/components/app/panel/tac.html",
            "<main>{basket.count}</main>",
        );
        write(
            root,
            "client/services/BasketService.js",
            "@Service()\nexport default class BasketService { count = 0 }",
        );
        write(
            root,
            "client/services/SearchService.ts",
            "@Service()\nexport default class SearchService {}",
        );
        write(
            root,
            "client/services/BasketService.swift",
            "@Service\nfinal class BasketService {}",
        );
        write(
            root,
            "client/services/BasketService.rs",
            "#[Service]\nstruct BasketService;",
        );
        write(
            root,
            "client/shared/scripts/tac.js",
            "export default class Stray {}",
        );
    }

    #[test]
    fn services_are_classified_by_language_and_stray_scripts_are_controllers() {
        let root = project();
        write_services_project(root.path());
        let report = check(root.path());
        for source in [
            "client/services/BasketService.js",
            "client/services/SearchService.ts",
        ] {
            let browser = with_source(&report, source);
            assert_eq!(browser.feature, "service.tac", "{source}");
            assert_eq!(browser.status, MigrationStatus::Supported, "{source}");
            assert!(browser.action.is_none(), "{source}");
        }
        let swift = with_source(&report, "client/services/BasketService.swift");
        assert_eq!(swift.feature, "service.native");
        assert_eq!(swift.status, MigrationStatus::Changed);
        assert!(swift.detail.contains("macos, ios"), "{swift:?}");
        assert!(
            action_contains(swift, "<Name>Service.js") && action_contains(swift, "ADR 0030"),
            "{swift:?}"
        );
        let rust = with_source(&report, "client/services/BasketService.rs");
        assert_eq!(rust.feature, "service.native");
        assert!(rust.detail.contains("macos, windows, linux"), "{rust:?}");
        let stray = with_source(&report, "client/shared/scripts/tac.js");
        assert_eq!(stray.feature, "companion.controller");
        assert_eq!(stray.status, MigrationStatus::Changed);
        assert!(
            action_contains(stray, "client/services/<Name>Service.js"),
            "{stray:?}"
        );
    }

    #[test]
    fn legacy_route_schemas_require_explicit_contract_migration() {
        let root = project();
        write(root.path(), "server/routes/users/OPTIONS.schema.json", "{}");
        let report = check(root.path());
        let finding = with_feature(&report, "server.route_schema");
        assert_eq!(finding.status, MigrationStatus::Changed);
        assert!(action_contains(finding, "model types"));
        assert!(action_contains(finding, "delete OPTIONS.schema.json"));
    }

    #[test]
    fn component_companions_move_to_services_and_polyglot_ones_stay_unsupported() {
        for (companion, feature, status) in [
            ("tac.js", "companion.component", MigrationStatus::Changed),
            ("tac.ts", "companion.component", MigrationStatus::Changed),
            ("tac.rs", "companion.component", MigrationStatus::Changed),
            ("tac.kt", "companion.component", MigrationStatus::Changed),
            ("tac.swift", "companion.component", MigrationStatus::Changed),
            ("tac.cs", "companion.component", MigrationStatus::Changed),
            (
                "tac.dart",
                "companion.polyglot",
                MigrationStatus::Unsupported,
            ),
            ("tac.py", "companion.polyglot", MigrationStatus::Unsupported),
        ] {
            let root = project();
            write(
                root.path(),
                "client/components/panel/tac.html",
                "<div>x</div>",
            );
            write(
                root.path(),
                &format!("client/components/panel/{companion}"),
                "x",
            );
            let report = check(root.path());
            let finding = with_source_ending(&report, companion);
            assert_eq!(finding.feature, feature, "{companion}");
            assert_eq!(finding.status, status, "{companion}");
            assert!(
                action_contains(finding, "client/services/"),
                "{companion} does not say where behaviour lives"
            );
        }
    }

    #[test]
    fn a_removed_wasm_sidecar_never_makes_a_component_supported() {
        let root = project();
        write(
            root.path(),
            "client/components/panel/tac.html",
            "<div>x</div>",
        );
        write(
            root.path(),
            "client/components/panel/tac.rs",
            "struct Legacy;",
        );
        write(
            root.path(),
            "client/components/panel/tachyon-wasm.rs",
            "fn real_compiler_source() {}",
        );
        let report = check(root.path());
        let finding = with_source_ending(&report, "tac.rs");
        assert_eq!(finding.feature, "companion.component");
        assert_eq!(finding.status, MigrationStatus::Changed);
        assert!(action_contains(finding, "client/services/PanelService.rs"));
        assert!(!report.is_clean());
    }

    #[test]
    fn a_missing_project_fails_closed() {
        let error = MigrationAnalysis::check(Path::new("/nonexistent/tachyon/project"))
            .expect_err("missing project");
        assert!(error.to_string().contains("TY1701"));
    }
}
