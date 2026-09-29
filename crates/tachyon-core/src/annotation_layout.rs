use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::fsharp_source::FsharpView;
use std::path::Path;

mod browser_interop;
mod file_name_check;
mod inline_model_check;
mod one_model_check;
mod single_class;
mod typed_members;

pub(crate) use browser_interop::{BrowserInteropCheck, NativeWebApis};
pub(crate) use file_name_check::FileNameCheck;
pub(crate) use inline_model_check::InlineModelCheck;
pub(crate) use one_model_check::OneModelCheck;
pub(crate) use single_class::cohesion::CohesionCheck;
pub(crate) use single_class::design_check::{
    DesignCheck, DesignSwitches, MissingType, UnusedMember,
};
pub(crate) use single_class::duplication::DuplicationCheck;
pub(crate) use single_class::rule_switch::RuleSwitch;
pub(crate) use single_class::simpler_forms::SimplerForm;
pub use single_class::simpler_forms::SimplerForms;
pub(crate) use single_class::utility_layout::UtilityLayout;

#[cfg(test)]
mod error_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod renamed_tests;

pub(crate) fn masked_source(path: &Path, source: &str) -> Result<String, Failure> {
    Input::new(path, source).masked_code()
}

pub(crate) fn declaration_names(path: &Path, source: &str) -> Result<Vec<String>, Failure> {
    FsharpView::either(path, source, declaration_names_source)
}

fn declaration_names_source(path: &Path, source: &str) -> Result<Vec<String>, Failure> {
    let code = masked_source(path, source)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let python = matches!(extension, "py" | "rb");
    let mut names = Vec::new();
    let mut depth = 0_usize;
    for line in code.lines() {
        if depth == 0
            && let Some(name) = top_level_declaration_name(extension, line, python)
        {
            names.push(name.to_owned());
        }
        if !python {
            depth = brace_depth_after(line, depth);
        }
    }
    Ok(names)
}

fn top_level_declaration_name<'a>(extension: &str, line: &'a str, python: bool) -> Option<&'a str> {
    let trimmed = line.trim_start();
    if extension == "go" {
        return Declaration::go_name(trimmed);
    }
    let indented = line.len() != trimmed.len();
    if python && indented {
        return None;
    }
    if Declaration::is_implementation(trimmed) {
        return None;
    }
    Declaration::name(trimmed)
}

pub(crate) fn declares_type(extension: &str, line: &str) -> bool {
    let line = line.trim_start();
    !Declaration::is_implementation(line) && Declaration::named(extension, line).is_some()
}

pub(crate) fn brace_depth_after(text: &str, mut depth: usize) -> usize {
    for byte in text.bytes() {
        match byte {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

pub(crate) fn validate(path: &Path, source: &str) -> Result<(), Failure> {
    crate::fsharp_source::FsharpStyle::check(path, source)?;
    FsharpView::either(path, source, validate_source)
}

fn validate_source(path: &Path, source: &str) -> Result<(), Failure> {
    reject_renamed_annotations(path, source)?;
    UtilityLayout::check(path, source)?;
    typed_members::TypedMembers::check(path, source)?;
    check_topics(path, source)?;
    let source = &crate::topics::Topics::blanked(source);
    if is_error_source(path) {
        single_class::validate_error_source(path, source)?;
    } else {
        primary_role(path, source)?;
        crate::model_layout::validate(path, source)?;
        OneModelCheck::check(path, source)?;
        InlineModelCheck::check(path, source)?;
        FileNameCheck::check(path, source)?;
    }
    crate::http_contract::HttpContract::check(path, source)?;
    crate::annotation_rules::AnnotationRules::check(path, source)
}

fn check_topics(path: &Path, source: &str) -> Result<(), Failure> {
    if crate::topics::Topics::markers(source).is_empty() {
        return Ok(());
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let declared = crate::route_types::ModelTable::declared_names(extension, source);
    crate::topics::Topics::check(path, source, &declared)
}

pub(crate) fn reject_renamed_annotations(path: &Path, source: &str) -> Result<(), Failure> {
    let input = Input::new(path, source);
    let code = input.masked_code()?;
    if input.extension == "go" {
        crate::go_source::validate_markers(path, source, &code)?;
    }
    if input.extension == "rb" {
        crate::ruby_source::RubyMarkers::validate(path, source, &code)?;
    }
    let mut offset = 0;
    for line in code.split_inclusive('\n') {
        let start = offset + line.len() - line.trim_start().len();
        offset += line.len();
        let text = line.trim();
        let Some(name) = marker_name(input.extension, text) else {
            continue;
        };
        let after_name = text.len() - text.trim_start_matches(['@', '#', '[']).len() + name.len();
        if text[after_name..].starts_with('.') {
            continue;
        }
        let portable = path.to_string_lossy().replace('\\', "/");
        if let Some(message) = crate::behavior::RenamedAnnotation::message(name) {
            return Err(Failure::one(diagnostic(
                1308,
                message,
                Some(String::from(
                    "Tachyon annotations have plain English names (ADR 0047). `ty migrate fix --write` renames them in every language.",
                )),
                source_span(&portable, start, (start + 1).min(source.len())),
            )));
        }
        if let Some(message) = crate::behavior::RoutingRename::message(name) {
            return Err(Failure::one(diagnostic(
                1308,
                message,
                Some(String::from(
                    "Routes and HTTP clients use annotation names close to Spring, and each one is explained in plain words (ADR 0048). `ty migrate fix --write` converts the old forms in every language.",
                )),
                source_span(&portable, start, (start + 1).min(source.len())),
            )));
        }
    }
    Ok(())
}

pub(crate) struct RenamedFolder;

impl RenamedFolder {
    pub(crate) const NAMES: [(&str, &str); 4] = [
        ("repositories", "stores"),
        ("configurations", "setups"),
        ("mappers", "converters"),
        ("delegates", "helpers"),
    ];

    pub(crate) fn new_name(realm: &str, folder: &str) -> Option<&'static str> {
        if realm != "client" && realm != "server" {
            return None;
        }
        for (old, new) in Self::NAMES {
            if old == folder {
                return Some(new);
            }
        }
        None
    }

    fn reject(path: &Path) -> Result<(), Failure> {
        let mut names = path
            .components()
            .map(|part| part.as_os_str().to_string_lossy());
        let realm = names.next().unwrap_or_default();
        let folder = names.next().unwrap_or_default();
        let Some(new_folder) = Self::new_name(&realm, &folder) else {
            return Ok(());
        };
        let portable = path.to_string_lossy().replace('\\', "/");
        Err(Failure::one(diagnostic(
            1310,
            format!(
                "'{portable}' is inside {realm}/{folder}, which was renamed to {realm}/{new_folder}"
            ),
            Some(format!(
                "Move the file to {realm}/{new_folder} (ADR 0047). `ty migrate fix --write` moves every file in a renamed role folder."
            )),
            source_span(&portable, 0, 0),
        )))
    }
}

pub(crate) struct RetiredRoutes;

impl RetiredRoutes {
    const ROUTES_HELP: &'static str = "Routes now come from annotations (ADR 0048): put each controller in server/controllers/ and mark it with @RequestRoute(\"/path\"), which gives the path its routes start with. `ty migrate fix --write` moves route files there.";
    const SCHEMA_HELP: &'static str = "Declare request and answer types in the controller method signature, with models from server/models/ (ADR 0048). The server answers OPTIONS with a schema built from them. Then delete this file.";

    fn reject(path: &Path) -> Result<(), Failure> {
        let mut names = path
            .components()
            .map(|part| part.as_os_str().to_string_lossy());
        if names.next().as_deref() != Some("server") {
            return Ok(());
        }
        let portable = path.to_string_lossy().replace('\\', "/");
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_default();
        if names.next().as_deref() == Some("routes") {
            let message =
                format!("'{portable}' is inside server/routes, which Tachyon no longer reads");
            return Err(Self::failure(&portable, message, Self::ROUTES_HELP));
        }
        if file_name == "OPTIONS.schema.json" {
            let message = format!(
                "'{portable}' is an OPTIONS.schema.json file, which Tachyon no longer reads"
            );
            return Err(Self::failure(&portable, message, Self::SCHEMA_HELP));
        }
        let route_file = path.file_stem().is_some_and(|stem| stem == "yon")
            && crate::convention::is_yon_language(path);
        if route_file {
            let message =
                format!("'{portable}' is a yon route file, which Tachyon no longer reads");
            return Err(Self::failure(&portable, message, Self::ROUTES_HELP));
        }
        Ok(())
    }

    fn failure(portable: &str, message: String, help: &str) -> Failure {
        Failure::one(diagnostic(
            1310,
            message,
            Some(String::from(help)),
            source_span(portable, 0, 0),
        ))
    }
}

pub(crate) fn reject_reserved_folder(path: &Path) -> Result<(), Failure> {
    RetiredRoutes::reject(path)?;
    RenamedFolder::reject(path)?;
    if !is_in_reserved_folder(path) {
        return Ok(());
    }
    let portable = path.to_string_lossy().replace('\\', "/");
    Err(Failure::one(diagnostic(
        1310,
        format!(
            "'{portable}' is inside server/gateways, which Tachyon reserves for generated gateway units"
        ),
        Some(String::from(
            "Move the file to a role folder such as server/services; a Gateway names its target file instead.",
        )),
        source_span(&portable, 0, 0),
    )))
}

fn is_in_reserved_folder(path: &Path) -> bool {
    let mut names = path
        .components()
        .map(|part| part.as_os_str().to_string_lossy());
    let first = names.next();
    let second = names.next();
    let in_server = first.is_some_and(|name| name.eq_ignore_ascii_case("server"));
    in_server && second.is_some_and(|name| name.eq_ignore_ascii_case("gateways"))
}

pub(crate) fn has_primary_role(path: &Path, source: &str) -> Result<bool, Failure> {
    FsharpView::either(path, source, primary_role)
}

pub(crate) fn has_framework_annotation(path: &Path, source: &str) -> Result<bool, Failure> {
    FsharpView::either(path, source, has_framework_annotation_source)
}

fn has_framework_annotation_source(path: &Path, source: &str) -> Result<bool, Failure> {
    reject_renamed_annotations(path, source)?;
    single_class::has_framework_annotation(path, source)
}

pub(crate) fn has_error_handler(path: &Path, source: &str) -> Result<bool, Failure> {
    FsharpView::either(path, source, |path, source| {
        single_class::has_role(path, source, "ErrorHandler")
    })
}

pub(crate) fn has_identity_provider(path: &Path, source: &str) -> Result<bool, Failure> {
    FsharpView::either(path, source, |path, source| {
        single_class::has_role(path, source, "IdentityProvider")
    })
}

pub(crate) fn has_route_controller(path: &Path, source: &str) -> Result<bool, Failure> {
    FsharpView::either(path, source, |path, source| {
        single_class::has_role(path, source, "RequestRoute")
    })
}

pub(crate) fn has_tested_controller(path: &Path, source: &str) -> Result<bool, Failure> {
    FsharpView::either(path, source, has_tested_controller_source)
}

fn has_tested_controller_source(path: &Path, source: &str) -> Result<bool, Failure> {
    let request = single_class::has_role(path, source, "RequestRoute")?;
    let socket = single_class::has_role(path, source, "SocketRoute")?;
    let error_handler = single_class::has_role(path, source, "ErrorHandler")?;
    let identity = single_class::has_role(path, source, "IdentityProvider")?;
    Ok([request, socket, error_handler, identity].contains(&true))
}

pub(crate) fn is_error_source(path: &Path) -> bool {
    path.starts_with("server/errors") || path.starts_with("client/errors")
}

pub(crate) fn has_test_annotation(path: &Path, source: &str) -> Result<bool, Failure> {
    FsharpView::either(path, source, single_class::has_test_annotation)
}

pub(crate) fn behaviour_types(path: &Path, source: &str) -> Result<Vec<String>, Failure> {
    FsharpView::either(path, source, behaviour_types_source)
}

fn behaviour_types_source(path: &Path, source: &str) -> Result<Vec<String>, Failure> {
    let scan = Scan::run(path, source)?;
    let mut names = Vec::new();
    for behaviour_type in scan.behaviour_types {
        names.push(behaviour_type.name);
    }
    Ok(names)
}

fn primary_role(path: &Path, source: &str) -> Result<bool, Failure> {
    single_class::validate(path, source)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Role {
    marker: &'static str,
    name: &'static str,
    folder: &'static str,
}

impl Role {
    fn named(name: &str) -> Option<Self> {
        let name = name
            .strip_prefix("yon::")
            .or_else(|| name.strip_prefix("tac::"))
            .unwrap_or(name);
        let (marker, name, folder) = match name.replace('_', "").to_ascii_lowercase().as_str() {
            "requestroute" => ("RequestRoute", "Controller", "controllers"),
            "socketroute" => ("SocketRoute", "Controller", "controllers"),
            "errorhandler" => ("ErrorHandler", "ErrorHandler", "controllers"),
            "identityprovider" => ("IdentityProvider", "IdentityProvider", "controllers"),
            "service" => ("Service", "Service", "services"),
            "store" => ("Store", "Store", "stores"),
            "client" => ("Client", "Client", "clients"),
            "httpclient" => ("HTTPClient", "Client", "clients"),
            "helper" => ("Helper", "Helper", "helpers"),
            "converter" => ("Converter", "Converter", "converters"),
            "setup" => ("Setup", "Setup", "setups"),
            "entity" => ("Entity", "Entity", "entities"),
            _ => return None,
        };
        Some(Self {
            marker,
            name,
            folder,
        })
    }

    fn is_controller(self) -> bool {
        self.folder == "controllers"
    }

    fn is_http_client(self) -> bool {
        self.marker == "HTTPClient"
    }

    fn has_suffix_in(self, type_name: &str) -> bool {
        type_name.len() > self.name.len() && type_name.ends_with(self.name)
    }

    fn expected_type_name(self, type_name: &str) -> String {
        if self.has_suffix_in(type_name) {
            return type_name.to_owned();
        }
        if type_name == self.name {
            return format!("Domain{}", self.name);
        }
        format!("{type_name}{}", self.name)
    }

    fn is_placed_in(self, parent: &str) -> bool {
        if self.is_controller() {
            return parent == "server/controllers";
        }
        parent == format!("client/{}", self.folder) || parent == format!("server/{}", self.folder)
    }
}

struct BehaviourType {
    name: String,
}

struct Input<'a> {
    path: &'a Path,
    source: &'a str,
    extension: &'a str,
}

impl<'a> Input<'a> {
    const MAX_CAPTURED_SOURCE: usize = 16 * 1024 * 1024;

    fn new(path: &'a Path, source: &'a str) -> Self {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        Self {
            path,
            source,
            extension,
        }
    }

    fn failure(&self, at: usize, message: impl Into<String>, help: impl Into<String>) -> Failure {
        let path = self.path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            1310,
            message,
            Some(help.into()),
            source_span(&path, at, (at + 1).min(self.source.len())),
        ))
    }

    fn parent_folder(&self) -> String {
        self.path
            .parent()
            .map(|parent| parent.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default()
    }

    fn check_placement(&self, role: Role, name: &str, at: usize) -> Result<(), Failure> {
        let expected_name = role.expected_type_name(name);
        self.check_file_name(role, name, &expected_name, at)?;
        self.check_folder(role, name, &expected_name, at)
    }

    fn check_file_name(
        &self,
        role: Role,
        name: &str,
        expected_name: &str,
        at: usize,
    ) -> Result<(), Failure> {
        let stem = self.path.file_stem().and_then(|value| value.to_str());
        if role.has_suffix_in(name) && stem == Some(name) {
            return Ok(());
        }
        let expected = format!("{}/{expected_name}.{}", role.folder, self.extension);
        Err(self.failure(
            at,
            format!("@{} type '{name}' does not match its required role layout", role.marker),
            format!("Name the type '{expected_name}' and place it in '{expected}' beneath the relevant client/server feature. The filename must exactly match the type, and its immediate parent must be '{}'.", role.folder),
        ))
    }

    fn check_folder(
        &self,
        role: Role,
        name: &str,
        expected_name: &str,
        at: usize,
    ) -> Result<(), Failure> {
        let parent = self.parent_folder();
        if role.is_placed_in(&parent) {
            return Ok(());
        }
        let destination = self.destination(role, &parent, expected_name);
        Err(self.failure(
            at,
            format!(
                "@{} type '{name}' sits outside its realm's shared role folder",
                role.marker
            ),
            format!(
                "Role folders are shared per realm (ADR 0031): move this file to '{destination}'."
            ),
        ))
    }

    fn destination(&self, role: Role, parent: &str, expected_name: &str) -> String {
        if role.is_controller() {
            return format!("server/controllers/{expected_name}.{}", self.extension);
        }
        let realm = if parent.starts_with("server") {
            "server"
        } else {
            "client"
        };
        format!("{realm}/{}/{expected_name}.{}", role.folder, self.extension)
    }

    fn masked_code(&self) -> Result<String, Failure> {
        if self.source.len() > Self::MAX_CAPTURED_SOURCE {
            return Err(self.failure(
                0,
                "Captured source exceeds 16 MiB",
                "Split source into bounded modules.",
            ));
        }
        let mut source = self.source.as_bytes().to_vec();
        if source.starts_with(&[0xef, 0xbb, 0xbf]) {
            source[..3].fill(b' ');
        }
        if matches!(self.extension, "js" | "ts") {
            mask_regex_literals(self.source, &mut source);
        }
        let source = String::from_utf8(source).unwrap_or_default();
        let mask = crate::lexical::code_mask(self.extension, &source);
        let mut code = source.into_bytes();
        for (byte, visible) in code.iter_mut().zip(mask.code) {
            if !visible && !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
        Ok(String::from_utf8(code).unwrap_or_default())
    }
}

struct Scan<'a> {
    input: Input<'a>,
    pending_role: Option<(Role, usize)>,
    behaviour_marked: bool,
    behaviour_types: Vec<BehaviourType>,
}

impl<'a> Scan<'a> {
    const MAX_SOURCE: usize = 1024 * 1024;

    fn run(path: &'a Path, source: &'a str) -> Result<Self, Failure> {
        let input = Input::new(path, source);
        let code = input.masked_code()?;
        let mut scan = Self {
            input,
            pending_role: None,
            behaviour_marked: false,
            behaviour_types: Vec::new(),
        };
        let mut offset = 0;
        for line in code.split_inclusive('\n') {
            let start = offset + line.len() - line.trim_start().len();
            offset += line.len();
            scan.read_line(line.trim(), start)?;
        }
        scan.finish()?;
        Ok(scan)
    }

    fn read_line(&mut self, text: &str, start: usize) -> Result<(), Failure> {
        if text.is_empty() {
            return Ok(());
        }
        if let Some(marker) = marker_name(self.input.extension, text) {
            return self.read_marker(marker, start);
        }
        self.read_code_line(text)?;
        self.behaviour_marked = false;
        Ok(())
    }

    fn read_marker(&mut self, marker: &str, start: usize) -> Result<(), Failure> {
        self.behaviour_marked |= is_behaviour_marker(marker);
        let Some(role) = Role::named(marker) else {
            return Ok(());
        };
        if self.input.source.len() > Self::MAX_SOURCE {
            return Err(self.input.failure(
                start,
                "Role-layout source exceeds 1 MiB",
                "Split source into bounded role modules.",
            ));
        }
        let Some((previous, _)) = self.pending_role else {
            self.pending_role = Some((role, start));
            return Ok(());
        };
        if role.is_http_client() || previous == role {
            return Ok(());
        }
        if previous.is_http_client() {
            self.pending_role = Some((role, start));
            return Ok(());
        }
        Err(self.input.failure(start, "A type cannot declare conflicting primary roles", "Keep one primary role annotation on each type; behavioral modifiers do not change its role."))
    }

    fn read_code_line(&mut self, text: &str) -> Result<(), Failure> {
        if let Some((_, at)) = self.pending_role.take() {
            return self.read_role_declaration(text, at);
        }
        if self.behaviour_marked
            && let Some(name) = Declaration::named(self.input.extension, text)
        {
            self.record_behaviour_type(name);
        }
        Ok(())
    }

    fn read_role_declaration(&mut self, text: &str, at: usize) -> Result<(), Failure> {
        let name = Declaration::named(self.input.extension, text).ok_or_else(|| self.input.failure(at, "A role annotation must precede a class, struct, object, or impl declaration", "Place each role annotation on its own line immediately before its type declaration."))?;
        self.record_behaviour_type(name);
        Ok(())
    }

    fn record_behaviour_type(&mut self, name: &str) {
        for previous in &self.behaviour_types {
            if previous.name == name {
                return;
            }
        }
        self.behaviour_types.push(BehaviourType {
            name: name.to_owned(),
        });
    }

    fn finish(&self) -> Result<(), Failure> {
        if let Some((_, at)) = self.pending_role {
            return Err(self.input.failure(
                at,
                "Role annotation has no following type",
                "Add the role-bearing type declaration after the annotation.",
            ));
        }
        Ok(())
    }
}

fn is_behaviour_marker(marker: &str) -> bool {
    let name = marker
        .rsplit("::")
        .next()
        .unwrap_or(marker)
        .replace('_', "")
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "requestroute"
            | "socketroute"
            | "httpclient"
            | "service"
            | "store"
            | "client"
            | "helper"
            | "converter"
            | "setup"
            | "cachesettings"
            | "lifetime"
    )
}

fn marker_name<'a>(extension: &str, line: &'a str) -> Option<&'a str> {
    let prefix = match extension {
        "rs" | "php" | "rb" => "#[",
        "cs" => "[",
        _ => "@",
    };
    let tail = line.strip_prefix(prefix)?;
    let length = tail
        .bytes()
        .take_while(|byte| is_identifier_byte(*byte) || *byte == b':')
        .count();
    Some(&tail[..length])
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

struct Declaration;

impl Declaration {
    const MAX_NAME: usize = 128;
    const MODIFIERS: [&'static str; 17] = [
        "public",
        "private",
        "protected",
        "internal",
        "final",
        "open",
        "abstract",
        "sealed",
        "static",
        "pub(crate)",
        "pub",
        "export",
        "default",
        "data",
        "partial",
        "readonly",
        "unsafe",
    ];
    const KINDS: [&'static str; 7] = [
        "class",
        "struct",
        "object",
        "record",
        "interface",
        "protocol",
        "trait",
    ];

    fn named<'a>(extension: &str, line: &'a str) -> Option<&'a str> {
        if extension == "go" {
            return Self::go_name(line);
        }
        Self::name(line)
    }

    fn go_name(line: &str) -> Option<&str> {
        let rest = Self::strip_word(line, "type")?;
        Self::take_name(rest)
    }

    fn is_implementation(line: &str) -> bool {
        line.split_whitespace()
            .any(|word| word == "impl" || word.starts_with("impl<"))
    }

    fn name(line: &str) -> Option<&str> {
        let line = Self::skip_modifiers(line);
        if let Some(rest) = Self::strip_impl(line) {
            return Self::implementation_target(rest.trim_start());
        }
        let rest = Self::KINDS
            .into_iter()
            .find_map(|word| Self::strip_word(line, word))?;
        Self::take_name(Self::skip_record_kind(rest))
    }

    fn skip_record_kind(rest: &str) -> &str {
        let kind = Self::strip_word(rest, "struct").or_else(|| Self::strip_word(rest, "class"));
        kind.unwrap_or(rest)
    }

    fn skip_modifiers(mut line: &str) -> &str {
        while let Some(rest) = Self::MODIFIERS
            .into_iter()
            .find_map(|word| Self::strip_word(line, word))
        {
            line = rest;
        }
        line
    }

    fn strip_impl(line: &str) -> Option<&str> {
        let rest = line.strip_prefix("impl")?;
        if rest.starts_with('<') || rest.starts_with(char::is_whitespace) {
            return Some(rest);
        }
        None
    }

    fn strip_word<'a>(line: &'a str, word: &str) -> Option<&'a str> {
        line.strip_prefix(word)
            .filter(|rest| rest.starts_with(char::is_whitespace))
            .map(str::trim_start)
    }

    fn take_name(source: &str) -> Option<&str> {
        let length = source
            .bytes()
            .take_while(|byte| is_identifier_byte(*byte))
            .count();
        let valid_end = source
            .as_bytes()
            .get(length)
            .is_none_or(|byte| byte.is_ascii() && *byte != b'$');
        if length == 0 || length > Self::MAX_NAME || !valid_end {
            return None;
        }
        Some(&source[..length])
    }

    fn implementation_target(mut source: &str) -> Option<&str> {
        if source.starts_with('<') {
            source = Self::skip_generics(source)?.trim_start();
        }
        let mut rest = source;
        let mut last_name = None;
        while !rest.is_empty() {
            if rest.starts_with(['{', ';']) || Self::strip_word(rest, "where").is_some() {
                break;
            }
            if rest.starts_with('<') {
                rest = Self::skip_generics(rest)?.trim_start();
                continue;
            }
            if let Some(after) = Self::strip_word(rest, "for") {
                last_name = None;
                rest = after;
                continue;
            }
            if let Some(after) = rest.strip_prefix("::") {
                rest = after.trim_start();
                continue;
            }
            let name = Self::take_name(rest)?;
            last_name = Some(name);
            rest = rest[name.len()..].trim_start();
        }
        last_name
    }

    fn skip_generics(source: &str) -> Option<&str> {
        let mut depth = 0_usize;
        for (at, byte) in source.bytes().enumerate() {
            match byte {
                b'<' => depth += 1,
                b'>' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(&source[at + 1..]);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

fn can_start_regex(before: &str) -> bool {
    let before = before.trim_end();
    if before.is_empty()
        || before.ends_with(['=', '(', '[', '{', ',', ':', ';', '!', '?', '&', '|'])
    {
        return true;
    }
    for keyword in ["return", "throw", "yield", "case"] {
        if ends_with_keyword(before, keyword) {
            return true;
        }
    }
    false
}

fn ends_with_keyword(text: &str, keyword: &str) -> bool {
    let Some(before_keyword) = text.strip_suffix(keyword) else {
        return false;
    };
    before_keyword
        .bytes()
        .next_back()
        .is_none_or(|byte| !is_identifier_byte(byte))
}

fn regex_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut at = start + 1;
    let mut in_bracket = false;
    while at < bytes.len() {
        match bytes[at] {
            b'\n' | b'\r' => return None,
            b'\\' => at += 1,
            b'[' => in_bracket = true,
            b']' => in_bracket = false,
            b'/' if !in_bracket => return Some(at + 1),
            _ => {}
        }
        at += 1;
    }
    None
}

fn mask_regex_literals(source: &str, output: &mut [u8]) {
    let mut at = 0;
    while at < output.len() {
        if matches!(output[at], b'\'' | b'"' | b'`') {
            at = quoted_end(output, at);
            continue;
        }
        if output.get(at..at + 2) == Some(b"//") {
            at += source[at..].find('\n').unwrap_or(source.len() - at);
            continue;
        }
        if output.get(at..at + 2) == Some(b"/*") {
            at = source[at + 2..]
                .find("*/")
                .map_or(source.len(), |offset| at + offset + 4);
            continue;
        }
        if output[at] == b'/'
            && can_start_regex(&source[..at])
            && let Some(end) = regex_end(source.as_bytes(), at)
        {
            output[at..end].fill(b' ');
            at = end;
            continue;
        }
        at += 1;
    }
}

fn quoted_end(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut at = start + 1;
    while at < bytes.len() {
        if bytes[at] == quote {
            return at + 1;
        }
        at += if bytes[at] == b'\\' { 2 } else { 1 };
    }
    bytes.len()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{behaviour_types, declaration_names, validate};
    use std::path::{Path, PathBuf};
    use tachyon_diagnostics::Diagnostic;

    fn assert_valid(path: &str, source: &str) {
        validate(Path::new(path), source).unwrap_or_else(|error| panic!("{path}: {error}"));
    }

    fn is_valid(path: &str, source: &str) -> bool {
        validate(Path::new(path), source).is_ok()
    }

    fn first_diagnostic(path: &str, source: &str) -> Diagnostic {
        let failure = validate(Path::new(path), source).expect_err(path);
        failure.diagnostics()[0].clone()
    }

    fn help_text(diagnostic: &Diagnostic) -> String {
        diagnostic.help.clone().unwrap_or_default()
    }

    #[test]
    fn equivalent_roles_validate_in_every_supported_language() {
        for (extension, source) in [
            ("js", "@Service\nexport class MoneyService {}"),
            ("ts", "@Service\nexport class MoneyService {}"),
            ("py", "@service\nclass MoneyService:\n    pass"),
            ("php", "<?php\n#[Service]\nfinal class MoneyService {}"),
            ("java", "@Service\npublic class MoneyService {}"),
            ("kt", "@Service\nclass MoneyService {}"),
            ("cs", "[Service]\npublic sealed class MoneyService {}"),
            ("rs", "#[yon::Service]\npub struct MoneyService;"),
            ("swift", "@Service\nfinal class MoneyService {}"),
            ("go", "// @yon::Service\ntype MoneyService struct{}"),
        ] {
            assert_valid(&format!("server/services/MoneyService.{extension}"), source);
            assert_valid(&format!("client/services/MoneyService.{extension}"), source);
            assert!(!is_valid(
                &format!("server/services/yon.{extension}"),
                source
            ));
        }
    }

    fn check_shared_role_folder(folder: &str, role: &str) {
        let marker = if role == "Client" { "HTTPClient" } else { role };
        let source = format!("@{marker}\nclass Money{role} {{}}");
        for realm in ["client", "server"] {
            assert_valid(&format!("{realm}/{folder}/Money{role}.ts"), &source);
        }
        let beside_route = format!("server/controllers/money/{folder}/Money{role}.ts");
        let diagnostic = first_diagnostic(&beside_route, &source);
        assert_eq!(diagnostic.code.number(), 1310);
        assert_eq!(
            diagnostic.message,
            format!("@{marker} type 'Money{role}' sits outside its realm's shared role folder")
        );
        let help = help_text(&diagnostic);
        assert!(
            help.contains(&format!("server/{folder}/Money{role}.ts")),
            "{help}"
        );
        assert!(help.contains("ADR 0031"), "{help}");
        for stray in [
            format!("server/features/payments/{folder}/Money{role}.ts"),
            format!("client/shared/{folder}/Money{role}.ts"),
            format!("client/pages/{folder}/Money{role}.ts"),
            format!("server/{folder}/nested/{folder}/Money{role}.ts"),
            format!("{folder}/Money{role}.ts"),
        ] {
            let failure = validate(Path::new(&stray), &source).expect_err(&stray);
            assert!(
                failure.to_string().contains("shared role folder"),
                "{stray}: {failure}"
            );
        }
        let client_side = format!("client/shared/{folder}/Money{role}.ts");
        let help = help_text(&first_diagnostic(&client_side, &source));
        assert!(
            help.contains(&format!("client/{folder}/Money{role}.ts")),
            "{help}"
        );
    }

    #[test]
    fn role_folders_are_shared_per_realm_and_controllers_live_in_server_controllers() {
        for (folder, role) in [
            ("services", "Service"),
            ("stores", "Store"),
            ("clients", "Client"),
            ("setups", "Setup"),
            ("entities", "Entity"),
            ("helpers", "Helper"),
            ("converters", "Converter"),
        ] {
            check_shared_role_folder(folder, role);
        }
        let controller = "@RequestRoute(\"/money\")\nclass MoneyController {}";
        assert_valid("server/controllers/MoneyController.ts", controller);
        for stray in [
            "client/controllers/MoneyController.ts",
            "server/controllers/nested/MoneyController.ts",
            "controllers/MoneyController.ts",
        ] {
            let help = help_text(&first_diagnostic(stray, controller));
            assert!(
                help.contains("server/controllers/MoneyController.ts"),
                "{stray}: {help}"
            );
        }
    }

    #[test]
    fn models_folders_hold_type_declarations_only() {
        let types = "import type { Section } from '/models/Section.ts'\n\n/** A guide. */\nexport interface Guide {\n  title: string;\n  sections: Section[]\n}\n\ninterface Pair<\n  Left,\n  Right\n> {\n  left: Left, right: Right\n}\n\nexport type Shape =\n  | { kind: 'circle' }\n  | { kind: 'square' }\n\ntype Handler = (event: Event) => void; type Name = string\ntype Choice<T> = T extends string\n  ? 'text'\n  : 'other'\ninterface Paged\n  extends Guide {}\nexport type { Section }\n";
        for realm in ["client", "server"] {
            let path = format!("{realm}/models/Guide.ts");
            crate::model_layout::validate(Path::new(&path), types)
                .unwrap_or_else(|error| panic!("{path}: {error}"));
            assert_eq!(
                first_diagnostic(&path, types).code.number(),
                crate::annotation_layout::FileNameCheck::CODE
            );
        }
        for (label, source) in [
            ("a class", "export class Guide {}\n"),
            (
                "a constant",
                "export interface Guide {}\nexport const empty = {}\n",
            ),
            ("a function", "function guide() {}\n"),
            ("an enum", "export enum Kind { Guide }\n"),
            (
                "a value import",
                "import { Section } from '/models/Section.ts'\n",
            ),
            (
                "a statement after a type",
                "type Name = string\nconsole.log('loaded')\n",
            ),
            (
                "a statement on the same line",
                "type Name = string; run()\n",
            ),
            ("a default export", "export default interface Guide {}\n"),
        ] {
            let diagnostic = first_diagnostic("client/models/Guide.ts", source);
            assert_eq!(diagnostic.code.number(), 1310, "{label}");
            assert_eq!(
                diagnostic.message, "A models file declares only interfaces and type aliases",
                "{label}"
            );
            assert!(
                help_text(&diagnostic).contains("client/services/"),
                "{label}"
            );
        }
        let diagnostic = first_diagnostic("server/models/Guide.js", "export class Guide {}\n");
        assert_eq!(diagnostic.code.number(), 1310);
        assert_eq!(
            diagnostic.message,
            "A JavaScript models file declares only JSDoc @typedef comments and `export {}`"
        );
        assert!(
            help_text(&diagnostic).contains("server/services/"),
            "{diagnostic:?}"
        );
        for outside in [
            "client/shared/models/Guide.ts",
            "client/pages/models/Guide.ts",
            "models/Guide.ts",
        ] {
            assert_valid(outside, "export const guide = {}\n");
        }
    }

    #[test]
    fn a_role_annotation_in_a_models_folder_names_its_role_folder() {
        for (realm, source) in [
            ("client", "@Service\nexport default class GuideService {}\n"),
            ("server", "@Service\nclass GuideService {}\n"),
        ] {
            let diagnostic = first_diagnostic(&format!("{realm}/models/GuideService.ts"), source);
            assert_eq!(diagnostic.code.number(), 1310);
            assert_eq!(
                diagnostic.message,
                "@Service type 'GuideService' sits outside its realm's shared role folder"
            );
            let help = help_text(&diagnostic);
            assert!(
                help.contains(&format!("{realm}/services/GuideService.ts")),
                "{help}"
            );
        }
    }

    #[test]
    fn composition_names_ignore_nested_types_strings_and_impl_blocks() {
        let rust = "struct MoneyService {\n field: u64,\n}\nimpl MoneyService {}\nimpl<T> Other<T> {}\nstruct Other<T>;";
        assert_eq!(
            declaration_names(Path::new("services/MoneyService.rs"), rust)
                .expect("Rust module declarations"),
            ["MoneyService", "Other"]
        );
        let javascript = "const text = `class Fake {}`;\nexport class MoneyService {\n method() {\n class Nested {}\n }\n}\nclass Handler {}";
        assert_eq!(
            declaration_names(Path::new("yon.js"), javascript)
                .expect("JavaScript module declarations"),
            ["MoneyService", "Handler"]
        );
        let python = "class Outer:\n    class Nested: pass\nclass Handler: pass";
        assert_eq!(
            declaration_names(Path::new("yon.py"), python).expect("Python module declarations"),
            ["Outer", "Handler"]
        );
    }

    fn is_annotated_language(path: &Path) -> bool {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        matches!(
            extension,
            "js" | "ts" | "py" | "php" | "java" | "kt" | "cs" | "rs" | "swift" | "dart"
        )
    }

    fn annotated_sources(roots: &[PathBuf]) -> Vec<PathBuf> {
        let mut pending = roots.to_vec();
        let mut found = Vec::new();
        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else if is_annotated_language(&path) {
                    found.push(path);
                }
            }
        }
        found.sort();
        found
    }

    fn declares_at_most_one_behaviour_type(path: &Path) -> bool {
        let Ok(source) = std::fs::read_to_string(path) else {
            return true;
        };
        behaviour_types(path, &source).is_ok_and(|behaviour| behaviour.len() < 2)
    }

    #[test]
    fn checked_in_fixtures_declare_one_behavior_bearing_type() {
        const NATIVE_UNIT: &str =
            "one native Tac unit: a cached class has no role, so no file of its own";
        const ASSEMBLED_ENTRY: &str = "one assembled Yon entry, as discovery composes it";
        const CONTRACT: &str = "a declaration contract tsc reads, never a project";
        const EXEMPT: [(&str, &str); 14] = [
            ("tests/fixtures/behavior/csharp-annotations.cs", NATIVE_UNIT),
            (
                "tests/fixtures/behavior/dart-annotations.dart",
                ASSEMBLED_ENTRY,
            ),
            (
                "tests/fixtures/behavior/dart-configuration.dart",
                ASSEMBLED_ENTRY,
            ),
            (
                "tests/fixtures/behavior/swift-annotations.swift",
                NATIVE_UNIT,
            ),
            ("tests/fixtures/behavior/providers.js", ASSEMBLED_ENTRY),
            (
                "tests/fixtures/behavior/rust_annotations.yon.rs",
                ASSEMBLED_ENTRY,
            ),
            ("../../types/tests/behavior-annotations.js", CONTRACT),
            ("../../types/tests/behavior-annotations.ts", CONTRACT),
            ("../../types/tests/behavior-parameters.ts", CONTRACT),
            ("../../types/tests/yon-annotations.js", CONTRACT),
            ("../../types/tests/yon-routes.ts", CONTRACT),
            ("../../types/tests/annotation-targets.ts", CONTRACT),
            ("../../types/tests/annotation-targets.js", CONTRACT),
            (
                "../../types/tests/yon-annotations.ts",
                "a declaration contract tsc reads, including shapes the compiler refuses",
            ),
        ];
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut unexpected = Vec::new();
        let mut unused: Vec<&str> = EXEMPT.iter().map(|(path, _)| *path).collect();
        let roots = [
            crate_root.join("tests/fixtures"),
            crate_root.join("../../types/tests"),
        ];
        for path in annotated_sources(&roots) {
            if declares_at_most_one_behaviour_type(&path) {
                continue;
            }
            let relative = path
                .strip_prefix(crate_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            match EXEMPT.iter().find(|(known, _)| *known == relative) {
                Some((known, _)) => unused.retain(|item| item != known),
                None => unexpected.push(format!("{relative} is not conformant")),
            }
        }
        assert!(
            unexpected.is_empty(),
            "split these sources, or name the reason in EXEMPT:\n{}",
            unexpected.join("\n")
        );
        assert!(
            unused.is_empty(),
            "EXEMPT lists sources that are conformant now; drop them: {unused:?}"
        );
    }

    #[test]
    fn a_role_file_registers_exactly_one_type() {
        let path = "server/services/OrderService.ts";
        for (label, source) in [
            (
                "a Checked companion",
                "@Checked()\nclass OrderInput {}\n@Service()\nclass OrderService {}\n",
            ),
            (
                "a Setting holder",
                "class ApplicationValues {\n@Setting(\"${yon.port:8080}\")\nport!: number;\n}\n@Service()\nclass OrderService {}\n",
            ),
            (
                "an unannotated helper",
                "@Service()\nclass OrderService {}\nclass Totals {}\n",
            ),
            (
                "a method-marked helper",
                "@Service()\nclass OrderService {}\nclass Totals {\n@PageDefaults({\"argument\":0})\npage(request: unknown) { return request; }\n}\n",
            ),
        ] {
            validate(Path::new(path), source).expect_err(label);
        }
        for (label, source) in [
            (
                "a second cache policy",
                "@Service()\nclass OrderService {}\n@CacheSettings({\"name\":\"orders\"})\nclass Totals {}\n",
            ),
            (
                "a second scope",
                "@Service()\nclass OrderService {}\n@Lifetime(\"transient\")\nclass Totals {}\n",
            ),
        ] {
            let diagnostic = first_diagnostic(path, source);
            assert_eq!(diagnostic.code.number(), 1310, "{label}");
            assert!(
                diagnostic.message.contains("one class"),
                "{label}: {}",
                diagnostic.message
            );
        }
        let entry = "@CacheSettings({\"name\":\"orders\"})\nclass Totals {}\n@CacheSettings({\"name\":\"audit\"})\nclass Audit {}\n";
        validate(Path::new("server/workers/orders.ts"), entry)
            .expect_err("authored entry is not a composed unit");
        let service = "@Service()\nexport default class OrdersService {}\n@CacheSettings({\"name\":\"orders\"})\nclass Totals {}\n";
        validate(Path::new("client/services/OrdersService.ts"), service)
            .expect_err("authored service is not a composed unit");
    }

    #[test]
    fn every_role_has_an_explicit_plural_and_exact_suffix() {
        for (marker, role, folder) in [
            ("RequestRoute", "Controller", "controllers"),
            ("SocketRoute", "Controller", "controllers"),
            ("Service", "Service", "services"),
            ("Store", "Store", "stores"),
            ("HTTPClient", "Client", "clients"),
            ("Helper", "Helper", "helpers"),
            ("Setup", "Setup", "setups"),
            ("Entity", "Entity", "entities"),
            ("Converter", "Converter", "converters"),
        ] {
            let path = if role == "Controller" {
                format!("server/{folder}/Money{role}.ts")
            } else {
                format!("client/{folder}/Money{role}.ts")
            };
            let source = format!("@{marker}\nclass Money{role} {{}}");
            assert!(is_valid(&path, &source));
            assert!(!is_valid(&path, &format!("@{marker}\nclass Money {{}}")));
            assert!(!is_valid(
                &format!("{folder}/{role}.ts"),
                &format!("@{marker}\nclass {role} {{}}")
            ));
            assert!(!is_valid(
                &format!("{folder}/nested/Money{role}.ts"),
                &source
            ));
        }
    }

    #[test]
    fn behavioral_modifiers_do_not_impose_roles() {
        for source in [
            "@Settings({})\nclass Settings {}",
            "@CacheSettings({})\n@NeedsAll()\nclass Ordinary {}",
            "@Description(\"A record\")\nclass Record {}",
        ] {
            assert!(is_valid("client/pages/tac.ts", source));
        }
        assert!(is_valid(
            "server/stores/MoneyStore.ts",
            "@Store(MoneyEntity)\nclass MoneyStore {}"
        ));
        assert!(is_valid(
            "client/entities/MoneyEntity.rs",
            "#[tac::entity({})]\nstruct MoneyEntity;"
        ));
        assert!(!is_valid(
            "client/pages/tac.rs",
            "#[tac::entity({})]\nstruct MoneyEntity;"
        ));
        assert!(is_valid(
            "server/services/MoneyService.ts",
            "@Service\n@CacheSettings({})\nclass MoneyService {}"
        ));
    }

    #[test]
    fn conflicting_and_multiple_primary_types_fail() {
        for source in [
            "@Service\n@Store\nclass MoneyService {}",
            "@Service\nclass MoneyService {}\n@Service\nclass OtherService {}",
            "@Service\nclass MoneyService {}\n@Service\nclass MoneyService {}",
            "@Service\nfunction MoneyService() {}",
            "@Service",
        ] {
            assert!(
                !is_valid("server/services/MoneyService.ts", source),
                "{source}"
            );
        }
    }

    #[test]
    fn rust_impl_uses_its_concrete_target_not_trait_or_generic() {
        let path = "server/services/MoneyService.rs";
        for source in [
            "struct MoneyService;\n#[Service]\nimpl MoneyService {}",
            "struct MoneyService<T>(T);\n#[Service]\nimpl<T> MoneyService<T> {}",
            "struct MoneyService<T>(T);\n#[Service]\nimpl<T> Trait<T> for MoneyService<T> {}",
            "#[Service]\nstruct MoneyService;\n#[Service]\nimpl MoneyService {}",
            "struct MoneyService;\n#[Service]\nimpl crate::MoneyService {}",
        ] {
            validate(Path::new(path), source).unwrap_or_else(|error| panic!("{source}: {error}"));
        }
        assert!(!is_valid(
            path,
            "#[Service]\nimpl MoneyService for Wrong {}"
        ));
    }

    #[test]
    fn strings_comments_and_regular_expressions_are_not_annotations() {
        for (extension, source) in [
            (
                "js",
                "const text = `\n@Service\nclass Wrong {}\n`;\n// @Service\n/*\n@Entity\nclass Wrong {}\n*/\nconst pattern = /[\"@Service]/;",
            ),
            (
                "ts",
                "@Service\nclass MoneyService { pattern = /[\"@Service]/; }",
            ),
            (
                "py",
                "text = '''\n@service\nclass Wrong: pass\n'''\n# @Service",
            ),
            (
                "rs",
                "const TEXT: &str = r###\"\n#[Service]\nstruct Wrong;\n\"###;",
            ),
            ("cs", "var text = @\"\n[Service]\nclass Wrong {}\n\";"),
            (
                "go",
                "// #[Service]\n/*\n#[Service]\ntype Wrong struct{}\n*/\n// @Service\ntype MoneyService struct {\n\tnote string `#[Service]`\n}",
            ),
        ] {
            let path = format!("server/services/MoneyService.{extension}");
            assert!(is_valid(&path, source), "{extension}");
        }
        assert!(
            !is_valid(
                "server/services/Wrong.ts",
                "const first = /\"/;\nconst second = /\"/;\n@Service\nclass MoneyService {}"
            ),
            "A regex quote must not hide a later role annotation"
        );
    }

    #[test]
    fn diagnostics_are_actionable_source_anchored_and_bounded() {
        let path = "server/other/NotMoney.ts";
        let diagnostic = first_diagnostic(path, "// intro\n@Service\nclass MoneyService {}");
        assert_eq!(diagnostic.code.number(), 1310);
        assert_eq!(diagnostic.spans[0].file, "server/other/NotMoney.ts");
        assert_eq!(diagnostic.spans[0].start, 9);
        assert!(help_text(&diagnostic).contains("services/MoneyService.ts"));
        assert!(is_valid(path, &" ".repeat(1024 * 1024 + 1)));
        let padded_role = format!(
            "{}\n@Service\nclass MoneyService {{}}",
            " ".repeat(1024 * 1024)
        );
        assert!(!is_valid(path, &padded_role));
        assert!(
            is_valid(path, "/* unterminated"),
            "Ordinary syntax diagnostics belong to the language frontend"
        );
        assert!(
            !is_valid(path, "\u{feff}@Service\nclass MoneyService {}"),
            "A UTF-8 BOM does not hide role validation"
        );
        for name in ["MoneyServiceé", "MoneyService$"] {
            assert!(
                !is_valid(
                    "server/services/MoneyService.ts",
                    &format!("@Service\nclass {name} {{}}")
                ),
                "Do not truncate an identifier to an apparently valid suffix"
            );
        }
    }
}
