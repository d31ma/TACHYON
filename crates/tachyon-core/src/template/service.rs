use super::ClientViewRenderer;
use super::required_fields::RequiredFields;
use crate::Failure;
use crate::compiler::hex_digest;
use crate::failure::{diagnostic, source_span};
use crate::fsharp_source::FsharpConstructorProps;
use crate::project::NativeCompanion;
use crate::tac_web::WebLanguage;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub(crate) struct ServiceRegistry {
    services: BTreeMap<String, ServiceDefinition>,
    digest: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ServiceDefinition {
    class: String,
    script: Option<PathBuf>,
    native: Vec<(NativeCompanion, PathBuf)>,
    go: Option<PathBuf>,
    ruby: Option<PathBuf>,
    lua: Option<PathBuf>,
    java: Option<PathBuf>,
    fsharp: Option<PathBuf>,
    prototype: bool,
    required: Vec<String>,
    injected: Vec<String>,
    stored: BTreeMap<String, String>,
    target: Option<tachyon_contracts::NativeTarget>,
}

impl ServiceDefinition {
    const fn new(class: String) -> Self {
        Self {
            class,
            script: None,
            native: Vec::new(),
            go: None,
            ruby: None,
            lua: None,
            java: None,
            fsharp: None,
            prototype: false,
            required: Vec::new(),
            injected: Vec::new(),
            stored: BTreeMap::new(),
            target: None,
        }
    }

    pub(crate) fn class(&self) -> &str {
        &self.class
    }

    pub(crate) fn script_path(&self) -> Option<&Path> {
        self.script.as_deref()
    }

    pub(crate) fn native_companions(&self) -> &[(NativeCompanion, PathBuf)] {
        &self.native
    }

    pub(crate) fn go_path(&self) -> Option<&Path> {
        self.go.as_deref()
    }

    pub(crate) const fn is_prototype(&self) -> bool {
        self.prototype
    }

    pub(crate) fn required(&self) -> &[String] {
        &self.required
    }

    pub(crate) fn injected(&self) -> &[String] {
        &self.injected
    }

    fn inject_native_capabilities(&mut self) {
        let names = self.native_capability_fields();
        self.required.retain(|name| !names.contains(name));
        let missing: Vec<String> = names
            .into_iter()
            .filter(|name| !self.injected.contains(name))
            .collect();
        self.injected.extend(missing);
    }

    fn native_capability_fields(&self) -> Vec<String> {
        marked_source(self)
            .and_then(|path| {
                fs::read_to_string(path)
                    .ok()
                    .map(|source| self.classify_native_capabilities(path, &source))
            })
            .unwrap_or_default()
    }

    fn classify_native_capabilities(&self, path: &Path, source: &str) -> Vec<String> {
        let types = BTreeSet::from([String::from("FilePicker"), String::from("WindowControls")]);
        let fields = RequiredFields::injected_fields(path, source, &self.class, &types);
        self.required
            .iter()
            .filter(|name| fields.contains(name))
            .cloned()
            .collect()
    }

    pub(crate) const fn stored(&self) -> &BTreeMap<String, String> {
        &self.stored
    }

    pub(crate) fn compiled(&self) -> bool {
        self.script.as_deref().is_none_or(|path| {
            path.extension()
                .is_some_and(|extension| extension == "dart")
        })
    }

    pub(crate) fn linked(&self) -> bool {
        let languages = self
            .native
            .iter()
            .map(|(language, _)| *language)
            .collect::<Vec<_>>();
        let Some(target) = self.target else {
            return false;
        };
        NativeCompanion::most_specific(&languages, target).is_some()
    }

    pub(crate) fn web_assembly(&self) -> Option<(WebLanguage, &Path)> {
        if self.script.is_some() || self.linked() {
            return None;
        }
        for wanted in WebLanguage::PREFERENCE {
            if let Some(path) = self.web_source(wanted) {
                return Some((wanted, path));
            }
        }
        None
    }

    fn web_source(&self, language: WebLanguage) -> Option<&Path> {
        let native = match language {
            WebLanguage::Go => return self.go.as_deref(),
            WebLanguage::Ruby => return self.ruby.as_deref(),
            WebLanguage::Lua => return self.lua.as_deref(),
            WebLanguage::Java => return self.java.as_deref(),
            WebLanguage::Rust => NativeCompanion::Rust,
            WebLanguage::CSharp => NativeCompanion::CSharp,
            WebLanguage::Kotlin => NativeCompanion::Kotlin,
            WebLanguage::Swift => NativeCompanion::Swift,
        };
        for (declared, path) in &self.native {
            if *declared == native {
                return Some(path.as_path());
            }
        }
        self.fsharp
            .as_deref()
            .filter(|_| language == WebLanguage::CSharp)
    }

    pub(crate) fn module_url(&self) -> Option<String> {
        if self.script.is_none() && self.web_assembly().is_none() {
            return None;
        }
        Some(format!("/.tachyon/services/{}.js", self.class))
    }
}

impl ServiceRegistry {
    const SERVICE_ROOT: &'static str = "client/services";
    const UTILITY_ROOT: &'static str = "client/utils";
    const MAX_SERVICES: usize = 256;
    const MAX_SERVICE_BYTES: u64 = 1_048_576;
    const PROVIDER_FOLDERS: [&'static str; 9] = [
        "client/stores",
        "client/remotes",
        "client/clients",
        "client/repositories",
        "client/configurations",
        "client/setups",
        "client/helpers",
        "client/converters",
        "client/entities",
    ];

    pub(crate) fn discover(project_root: &Path) -> Result<Self, Failure> {
        let paths = Self::service_paths(project_root)?;
        let mut diagnostics = Vec::new();
        let mut services = BTreeMap::<String, ServiceDefinition>::new();
        let mut hasher = Sha256::new();
        let mut variants = BTreeSet::new();
        for path in paths {
            let relative = path
                .strip_prefix(project_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if let Some((class, extension)) = service_file_name(&path)
                && !variants.insert((class.clone(), extension.to_owned()))
            {
                diagnostics.push(service_diagnostic(&relative, &format!("Service '{class}' declares more than one {extension} file."), "Nested folders organize services; keep one file per service and language across the realm."));
                continue;
            }
            let added = add_service_file(&path, &relative, &mut services, &mut diagnostics);
            if added {
                hash_file(&mut hasher, &relative, &path);
            }
        }
        Self::hash_utilities(project_root, &mut hasher);
        set_native_service_scopes(&mut services);
        set_required_fields(&mut services, Self::provider_names(project_root)?);
        set_stored_fields(&mut services);
        if services.len() > Self::MAX_SERVICES {
            diagnostics.push(service_diagnostic(
                Self::SERVICE_ROOT,
                "Service registry exceeds the limit of 256.",
                "Split the application or remove unused services.",
            ));
        }
        if !diagnostics.is_empty() {
            return Err(Failure::new(diagnostics));
        }
        Ok(Self {
            services,
            digest: hex_digest(hasher.finalize()),
        })
    }

    fn hash_utilities(project_root: &Path, hasher: &mut Sha256) {
        let mut paths: Vec<PathBuf> = fs::read_dir(project_root.join(Self::UTILITY_ROOT))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .collect();
        paths.sort();
        for path in paths {
            let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
            hash_file(
                hasher,
                &format!("{}/{file_name}", Self::UTILITY_ROOT),
                &path,
            );
        }
    }

    fn provider_names(project_root: &Path) -> Result<BTreeSet<String>, Failure> {
        let mut names = BTreeSet::new();
        for folder in Self::PROVIDER_FOLDERS {
            for path in
                crate::annotation_modules::Discovery::role_paths(project_root, Path::new(folder))?
            {
                if let Some(class) = path.file_stem().and_then(OsStr::to_str) {
                    names.insert(crate::companion::lower_first(class));
                    names.insert(class.to_owned());
                }
            }
        }
        Ok(names)
    }

    fn service_paths(project_root: &Path) -> Result<Vec<PathBuf>, Failure> {
        crate::annotation_modules::Discovery::role_paths(
            project_root,
            Path::new(Self::SERVICE_ROOT),
        )
        .map(|paths| {
            paths
                .into_iter()
                .map(|path| project_root.join(path))
                .collect()
        })
    }

    pub(crate) fn for_target(mut self, target: Option<tachyon_contracts::NativeTarget>) -> Self {
        self.services
            .values_mut()
            .for_each(|service| service.target = target);
        self.services
            .values_mut()
            .filter(|service| service.script.is_none() && service.linked())
            .for_each(ServiceDefinition::inject_native_capabilities);
        self
    }

    pub(crate) fn names(&self) -> BTreeSet<String> {
        self.services.keys().cloned().collect()
    }

    pub(crate) fn get(&self, instance: &str) -> Option<&ServiceDefinition> {
        self.services.get(instance)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &ServiceDefinition)> {
        self.services.iter()
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }
}

fn add_service_file(
    path: &Path,
    relative: &str,
    services: &mut BTreeMap<String, ServiceDefinition>,
    diagnostics: &mut Vec<tachyon_diagnostics::Diagnostic>,
) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        diagnostics.push(service_diagnostic(
            relative,
            "Services must be regular files beneath client/services, never symlinks.",
            "Move collaborators to client/remotes, client/stores, client/setups, \
             client/entities or client/helpers; a symlink is never a service.",
        ));
        return false;
    }
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");
    if ![
        "js", "ts", "dart", "swift", "kt", "java", "cs", "fs", "rs", "go", "rb", "lua",
    ]
    .contains(&extension)
    {
        return false;
    }
    if metadata.len() > ServiceRegistry::MAX_SERVICE_BYTES {
        diagnostics.push(service_diagnostic(
            relative,
            "A service source exceeds the 1 MiB limit.",
            "Split the service into bounded role files.",
        ));
        return false;
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            diagnostics.push(service_diagnostic(
                relative,
                &format!("Cannot read service source: {error}"),
                "Use readable UTF-8 source.",
            ));
            return false;
        }
    };
    let relative_path = Path::new(relative);
    if let Err(failure) = crate::annotation_layout::FolderRoleCheck::check(relative_path, &source) {
        diagnostics.extend_from_slice(failure.diagnostics());
        return false;
    }
    if matches!(
        crate::annotation_layout::FolderRoleCheck::is_candidate(relative_path, &source),
        Ok(false)
    ) {
        return false;
    }
    let Some((class, extension)) = service_file_name(path) else {
        diagnostics.push(service_diagnostic(
            relative,
            "A service file is named <Name>Service with a js, ts, dart, swift, kt, java, cs, fs, rs, go, rb or lua extension.",
            "Rename it, for example client/services/BasketService.js declaring class BasketService.",
        ));
        return false;
    };
    let instance = instance_name(&class);
    if let Some(reserved) = view_root(relative, &instance) {
        diagnostics.push(reserved);
        return false;
    }
    let definition = services
        .entry(instance)
        .or_insert_with(|| ServiceDefinition::new(class.clone()));
    if matches!(extension, "js" | "ts" | "dart") {
        return add_browser_file(definition, path, relative, &class, extension, diagnostics);
    }
    if extension == "go" {
        definition.go = Some(path.to_path_buf());
        return true;
    }
    if extension == "rb" {
        definition.ruby = Some(path.to_path_buf());
        return true;
    }
    if extension == "java" {
        definition.java = Some(path.to_path_buf());
        return true;
    }
    if extension == crate::lua_source::LuaSource::EXTENSION {
        definition.lua = Some(path.to_path_buf());
        return true;
    }
    if extension == crate::fsharp_source::FsharpSource::EXTENSION {
        definition.fsharp = Some(path.to_path_buf());
        return true;
    }
    let Some(language) = native_language(extension) else {
        return false;
    };
    definition.native.push((language, path.to_path_buf()));
    true
}

fn add_browser_file(
    definition: &mut ServiceDefinition,
    path: &Path,
    relative: &str,
    class: &str,
    extension: &str,
    diagnostics: &mut Vec<tachyon_diagnostics::Diagnostic>,
) -> bool {
    if definition.script.is_some() {
        diagnostics.push(service_diagnostic(
            relative,
            &format!("Service '{class}' declares more than one browser file (js, ts or dart)."),
            "Keep one browser language per service.",
        ));
        return false;
    }
    if let Some(prototype) = inspect_script(path, relative, class, extension, diagnostics) {
        definition.prototype = prototype;
        definition.script = Some(path.to_path_buf());
    }
    true
}

fn view_root(relative: &str, instance: &str) -> Option<tachyon_diagnostics::Diagnostic> {
    if !ClientViewRenderer::VIEW_ROOTS.contains(&instance) {
        return None;
    }
    Some(service_diagnostic(
        relative,
        &format!(
            "Every view has `{instance}` built in (ADR 0036), so no service is addressed as `{instance}`."
        ),
        "Rename the service, for example DatasetService or PagesService.",
    ))
}

fn set_native_service_scopes(services: &mut BTreeMap<String, ServiceDefinition>) {
    for definition in services.values_mut() {
        if definition.script.is_some() {
            continue;
        }
        let Some(path) = marked_source(definition) else {
            continue;
        };
        if let Ok(source) = fs::read_to_string(path) {
            let roles = crate::behavior::ServiceRoles::read(path, &source);
            definition.prototype = roles.is_ok_and(|roles| roles.prototype)
                || FsharpConstructorProps::takes_props(path, &source);
        }
    }
}

fn set_required_fields(
    services: &mut BTreeMap<String, ServiceDefinition>,
    mut injectable: BTreeSet<String>,
) {
    for (instance, definition) in services.iter() {
        injectable.insert(instance.clone());
        injectable.insert(definition.class.clone());
    }
    for definition in services.values_mut() {
        let Some(path) = definition
            .script
            .as_deref()
            .or_else(|| marked_source(definition))
        else {
            continue;
        };
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        let needs_all = crate::behavior::ServiceRoles::read(path, &source)
            .is_ok_and(|roles| !roles.required.is_empty())
            || FsharpConstructorProps::takes_props(path, &source);
        if needs_all {
            let class = &definition.class;
            let props = RequiredFields::props(path, &source, class, &injectable);
            let injected = RequiredFields::injected_fields(path, &source, class, &injectable);
            definition.required = props;
            definition.injected = injected;
        }
    }
}

fn set_stored_fields(services: &mut BTreeMap<String, ServiceDefinition>) {
    for definition in services.values_mut() {
        let Some(path) = definition
            .script
            .as_deref()
            .or_else(|| marked_source(definition))
        else {
            continue;
        };
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        if let Ok(stored) = crate::behavior::StoredState::fields(path, &source) {
            definition.stored = stored;
        }
    }
}

fn marked_source(definition: &ServiceDefinition) -> Option<&Path> {
    for (language, path) in &definition.native {
        if matches!(language, NativeCompanion::Rust | NativeCompanion::CSharp) {
            return Some(path);
        }
    }
    if let Some(go) = &definition.go {
        return Some(go);
    }
    if let Some(ruby) = &definition.ruby {
        return Some(ruby);
    }
    if let Some(java) = &definition.java {
        return Some(java);
    }
    if let Some(fsharp) = &definition.fsharp {
        return Some(fsharp);
    }
    if let Some(lua) = &definition.lua {
        return Some(lua);
    }
    let (_, path) = definition.native.first()?;
    Some(path)
}

pub(crate) fn instance_name(class: &str) -> String {
    let stem = class.strip_suffix("Service").unwrap_or(class);
    let mut characters = stem.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    first.to_ascii_lowercase().to_string() + characters.as_str()
}

fn service_file_name(path: &Path) -> Option<(String, &'static str)> {
    let stem = path.file_stem()?.to_str()?;
    let extension = match path.extension()?.to_str()? {
        "js" => "js",
        "ts" => "ts",
        "dart" => "dart",
        "swift" => "swift",
        "kt" => "kt",
        "java" => "java",
        "cs" => "cs",
        "fs" => "fs",
        "rs" => "rs",
        "go" => "go",
        "rb" => "rb",
        "lua" => "lua",
        _ => return None,
    };
    let name = stem.strip_suffix("Service")?;
    let starts_with_capital = name
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_uppercase());
    let is_alphanumeric = name.bytes().all(|byte| byte.is_ascii_alphanumeric());
    let is_short = stem.len() <= 96;
    if !starts_with_capital || !is_alphanumeric || !is_short {
        return None;
    }
    Some((stem.to_owned(), extension))
}

fn native_language(extension: &str) -> Option<NativeCompanion> {
    match extension {
        "swift" => Some(NativeCompanion::Swift),
        "kt" => Some(NativeCompanion::Kotlin),
        "cs" => Some(NativeCompanion::CSharp),
        "rs" => Some(NativeCompanion::Rust),
        _ => None,
    }
}

fn inspect_script(
    path: &Path,
    relative: &str,
    class: &str,
    extension: &str,
    diagnostics: &mut Vec<tachyon_diagnostics::Diagnostic>,
) -> Option<bool> {
    let source = read_service_source(path, relative, diagnostics)?;
    let is_dart = extension == "dart";
    let mask_language = if is_dart { "dart" } else { "js" };
    let mask = crate::lexical::code_mask(mask_language, &source);
    let code = &mask.code;
    if is_dart && !declares_dart_class(&source, code, class) {
        diagnostics.push(service_diagnostic(
            relative,
            &format!("A Dart service declares class {class}."),
            &format!("Write `class {class} {{ ... }}` in {relative}."),
        ));
        return None;
    }
    if !is_dart && !marker_in_code(&source, code, "export default class") {
        diagnostics.push(service_diagnostic(
            relative,
            "A service default-exports its class.",
            "Write `export default class <Name>Service { ... }`.",
        ));
        return None;
    }
    Some(is_prototype_scope(&source, code))
}

fn read_service_source(
    path: &Path,
    relative: &str,
    diagnostics: &mut Vec<tachyon_diagnostics::Diagnostic>,
) -> Option<String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            diagnostics.push(service_diagnostic(
                relative,
                &format!("Cannot read service source: {error}"),
                "Check the file permissions.",
            ));
            return None;
        }
    };
    if bytes.len() as u64 > ServiceRegistry::MAX_SERVICE_BYTES {
        diagnostics.push(service_diagnostic(
            relative,
            "Service source exceeds the 1 MiB limit.",
            "Split the service.",
        ));
        return None;
    }
    match String::from_utf8(bytes) {
        Ok(source) if !source.contains('\0') => Some(source),
        _ => {
            diagnostics.push(service_diagnostic(
                relative,
                "Service source must be NUL-free UTF-8.",
                "Save the file as UTF-8.",
            ));
            None
        }
    }
}

fn declares_dart_class(source: &str, code: &[bool], class: &str) -> bool {
    marker_in_code(source, code, &format!("class {class} "))
        || marker_in_code(source, code, &format!("class {class}{{"))
}

fn is_prototype_scope(source: &str, code: &[bool]) -> bool {
    for (at, _) in source.match_indices("@Lifetime(") {
        if !is_code(code, at) {
            continue;
        }
        let scope_text = source[at..].chars().take(64).collect::<String>();
        return scope_text.contains("prototype");
    }
    marker_in_code(source, code, "@NeedsAll")
}

fn marker_in_code(source: &str, code: &[bool], marker: &str) -> bool {
    source
        .match_indices(marker)
        .any(|(at, _)| is_code(code, at))
}

fn is_code(code: &[bool], at: usize) -> bool {
    code.get(at) == Some(&true)
}

fn hash_file(hasher: &mut Sha256, relative: &str, path: &Path) {
    hasher.update(relative.as_bytes());
    hasher.update([0]);
    if let Ok(bytes) = fs::read(path) {
        hasher.update(bytes);
    }
    hasher.update([0]);
}

fn service_diagnostic(
    relative: &str,
    message: &str,
    help: &str,
) -> tachyon_diagnostics::Diagnostic {
    diagnostic(
        1310,
        format!("Components use services: {message}"),
        Some(String::from(help)),
        source_span(relative, 0, relative.len()),
    )
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn project(files: &[(&str, &str)]) -> TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, contents) in files {
            let path = root.path().join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            fs::write(path, contents).expect("write");
        }
        root
    }

    fn discover_error(files: &[(&str, &str)], reason: &str) -> Failure {
        let root = project(files);
        ServiceRegistry::discover(root.path()).expect_err(reason)
    }

    #[test]
    fn instance_names_drop_the_suffix_and_lower_the_first_letter() {
        assert_eq!(instance_name("BasketService"), "basket");
        assert_eq!(instance_name("UserListService"), "userList");
        assert_eq!(instance_name("HTTPService"), "hTTP");
    }

    #[test]
    fn services_are_discovered_with_their_browser_and_native_files() {
        let root = project(&[
            (
                "client/services/BasketService.js",
                "@Service()\nexport default class BasketService { count = 0 }",
            ),
            (
                "client/services/BasketService.swift",
                "@Service\nfinal class BasketService {}",
            ),
            (
                "client/services/SnippetService.ts",
                "@Service()\n@Lifetime({\"value\":\"prototype\"})\nexport default class SnippetService {}",
            ),
            (
                "client/services/TocService.js",
                "@Service\n@NeedsAll\nexport default class TocService { headings }",
            ),
            (
                "client/services/PagerService.js",
                "@Service\n@Lifetime({\"value\":\"singleton\"})\n@NeedsAll\nexport default class PagerService { path }",
            ),
        ]);
        let registry = ServiceRegistry::discover(root.path()).expect("registry");
        assert!(registry.get("toc").expect("toc").is_prototype());
        assert!(!registry.get("pager").expect("pager").is_prototype());
        assert_eq!(
            registry.names(),
            BTreeSet::from(["basket", "pager", "snippet", "toc"].map(String::from))
        );
        let basket = registry.get("basket").expect("basket");
        assert_eq!(basket.class(), "BasketService");
        assert!(basket.script_path().is_some());
        assert_eq!(basket.native_companions().len(), 1);
        assert!(!basket.is_prototype());
        assert_eq!(
            basket.module_url().as_deref(),
            Some("/.tachyon/services/BasketService.js")
        );
        assert!(registry.get("snippet").expect("snippet").is_prototype());
        assert!(!registry.digest().is_empty());
    }

    #[test]
    fn a_rust_only_service_is_webassembly_wherever_no_host_links_it() {
        use tachyon_contracts::NativeTarget;
        let root = project(&[
            (
                "client/services/ClockService.rs",
                "#[derive(Default)]\n#[Service]\nstruct ClockService {}\n",
            ),
            (
                "client/services/BasketService.js",
                "@Service\nexport default class BasketService {}",
            ),
            (
                "client/services/BasketService.rs",
                "#[derive(Default)]\n#[Service]\nstruct BasketService {}\n",
            ),
        ]);
        let registry = ServiceRegistry::discover(root.path()).expect("registry");
        for (target, linked) in [
            (None, false),
            (Some(NativeTarget::Ios), false),
            (Some(NativeTarget::Android), false),
            (Some(NativeTarget::Macos), true),
            (Some(NativeTarget::Linux), true),
        ] {
            let registry = registry.clone().for_target(target);
            let clock = registry.get("clock").expect("clock");
            assert_eq!(clock.linked(), linked, "{target:?}");
            assert_eq!(clock.web_assembly().is_some(), !linked, "{target:?}");
            assert_eq!(
                clock.module_url().is_some(),
                !linked,
                "a linked service is never imported: {target:?}"
            );
            let basket = registry.get("basket").expect("basket");
            assert!(basket.web_assembly().is_none());
            assert!(basket.module_url().is_some());
        }
    }

    #[test]
    fn no_service_is_addressed_as_a_view_root() {
        for class in ["DataService", "PageService"] {
            let relative = format!("client/services/{class}.js");
            let contents = format!("@Service\nexport default class {class} {{}}");
            let error = discover_error(&[(&relative, &contents)], class);
            assert!(
                error.diagnostics()[0].message.contains("built in"),
                "{class}"
            );
        }
    }

    #[test]
    fn a_missing_services_directory_is_an_empty_registry() {
        let root = project(&[]);
        let registry = ServiceRegistry::discover(root.path()).expect("registry");
        assert!(registry.names().is_empty());
    }

    #[test]
    fn a_misnamed_service_file_is_refused() {
        let error = discover_error(
            &[(
                "client/services/basket.js",
                "@Service\nexport default class basket {}",
            )],
            "misnamed",
        );
        assert_eq!(error.diagnostics()[0].code.number(), 1310);
    }

    #[test]
    fn a_go_only_service_is_webassembly_on_every_target() {
        use tachyon_contracts::NativeTarget;
        let root = project(&[(
            "client/services/BasketService.go",
            "// @Service\n// @Lifetime({\"value\":\"prototype\"})\ntype BasketService struct{ Count int }\n",
        )]);
        let registry = ServiceRegistry::discover(root.path()).expect("registry");
        for target in [None, Some(NativeTarget::Windows), Some(NativeTarget::Macos)] {
            let registry = registry.clone().for_target(target);
            let basket = registry.get("basket").expect("basket");
            assert!(basket.native_companions().is_empty(), "{target:?}");
            assert!(!basket.linked(), "{target:?}");
            let (language, path) = basket.web_assembly().expect("web assembly");
            assert_eq!(language, WebLanguage::Go);
            assert!(path.ends_with("BasketService.go"));
            assert_eq!(
                basket.module_url().as_deref(),
                Some("/.tachyon/services/BasketService.js")
            );
            assert!(basket.is_prototype());
        }
    }

    #[test]
    fn rust_comes_before_go_and_go_before_csharp_on_the_web() {
        let root = project(&[
            (
                "client/services/ClockService.go",
                "// @Service\ntype ClockService struct{}\n",
            ),
            (
                "client/services/ClockService.rs",
                "#[derive(Default)]\n#[Service]\nstruct ClockService {}\n",
            ),
            (
                "client/services/TallyService.go",
                "// @Service\ntype TallyService struct{}\n",
            ),
            (
                "client/services/TallyService.cs",
                "[Service]\npublic class TallyService {}\n",
            ),
        ]);
        let registry = ServiceRegistry::discover(root.path()).expect("registry");
        let clock = registry.get("clock").expect("clock");
        assert_eq!(clock.web_assembly().expect("clock").0, WebLanguage::Rust);
        let tally = registry.get("tally").expect("tally");
        assert_eq!(tally.web_assembly().expect("tally").0, WebLanguage::Go);
        let windows = registry.for_target(Some(tachyon_contracts::NativeTarget::Windows));
        assert!(
            windows
                .get("tally")
                .expect("tally")
                .web_assembly()
                .is_none()
        );
    }

    #[test]
    fn go_is_never_read_as_a_native_companion() {
        assert!(native_language("go").is_none());
        assert_eq!(native_language("rs"), Some(NativeCompanion::Rust));
    }

    #[test]
    fn two_browser_languages_for_one_service_are_refused() {
        let error = discover_error(
            &[
                (
                    "client/services/BasketService.js",
                    "@Service\nexport default class BasketService {}",
                ),
                (
                    "client/services/BasketService.ts",
                    "@Service\nexport default class BasketService {}",
                ),
            ],
            "two languages",
        );
        assert!(
            error.diagnostics()[0]
                .message
                .contains("more than one browser file")
        );

        let root = project(&[
            (
                "client/services/BasketService.js",
                "@Service\nexport default class BasketService {}",
            ),
            (
                "client/services/BasketService.dart",
                "@Service\nclass BasketService {}",
            ),
        ]);
        assert!(ServiceRegistry::discover(root.path()).is_err());
    }

    #[test]
    fn a_dart_service_declares_its_class() {
        let error = discover_error(
            &[(
                "client/services/BasketService.dart",
                "@Service\nclass Basket {}",
            )],
            "wrong Dart class",
        );
        assert!(
            error.diagnostics()[0]
                .message
                .contains("required role layout")
        );

        let root = project(&[(
            "client/services/BasketService.dart",
            "@Service()\n@Lifetime({\"value\":\"prototype\"})\nclass BasketService {\n  int count = 0;\n}\n",
        )]);
        let registry = ServiceRegistry::discover(root.path()).expect("Dart service");
        let basket = registry.get("basket").expect("basket");
        assert!(basket.is_prototype());
        assert_eq!(
            basket.module_url().as_deref(),
            Some("/.tachyon/services/BasketService.js")
        );
    }

    #[test]
    fn java_and_fsharp_needs_all_services_take_their_component_props() {
        let root = project(&[
            (
                "client/services/ShelfService.java",
                "import java.util.List;\n\n@Service\n@NeedsAll\nclass ShelfService {\n    List<String> files;\n    String owner;\n    int count = 0;\n}\n",
            ),
            (
                "client/services/SnippetService.fs",
                "[<Service>]\n[<NeedsAll>]\ntype SnippetService(files: string list, owner: string) =\n    member this.Owner : string = owner\n",
            ),
        ]);
        let registry = ServiceRegistry::discover(root.path()).expect("Java and F# services");
        for instance in ["shelf", "snippet"] {
            let service = registry.get(instance).expect(instance);
            assert_eq!(service.required(), ["files", "owner"], "{instance}");
            assert!(service.is_prototype(), "{instance}");
        }
    }

    #[test]
    fn a_native_capability_is_injected_instead_of_a_component_prop() {
        use tachyon_contracts::NativeTarget;
        let variants = [
            (
                "client/services/PickerService.swift",
                NativeTarget::Macos,
                "@Service\n@NeedsAll\nfinal class PickerService {\n private let filePicker: FilePicker\n private let windowControls: WindowControls\n let title: String\n}\n",
            ),
            (
                "client/services/PickerService.cs",
                NativeTarget::Windows,
                "[Service]\n[NeedsAll]\npublic class PickerService {\n public readonly FilePicker FilePicker;\n public readonly WindowControls WindowControls;\n public readonly string Title;\n}\n",
            ),
            (
                "client/services/PickerService.kt",
                NativeTarget::Android,
                "@Service\n@NeedsAll\nclass PickerService {\n val filePicker: FilePicker\n val windowControls: WindowControls\n val title: String\n}\n",
            ),
        ];
        for (path, target, source) in variants {
            let root = project(&[(path, source)]);
            let registry = ServiceRegistry::discover(root.path()).expect("service");
            assert_eq!(
                registry.get("picker").expect("web").required(),
                ["filePicker", "windowControls", "title"],
                "{path}"
            );
            let native = registry.for_target(Some(target));
            let service = native.get("picker").expect("native");
            assert_eq!(service.required(), ["title"], "{path}");
            assert_eq!(
                service.injected(),
                ["filePicker", "windowControls"],
                "{path}"
            );
        }
    }

    #[test]
    fn a_script_service_without_a_default_export_is_refused() {
        let error = discover_error(
            &[(
                "client/services/BasketService.js",
                "@Service\nexport class BasketService {}",
            )],
            "no default export",
        );
        assert!(error.diagnostics()[0].message.contains("default-exports"));
    }

    #[test]
    fn nested_services_retain_their_paths_and_injected_providers() {
        let root = project(&[
            (
                "client/services/nested/DeepService.js",
                "@Service\n@NeedsAll\nexport default class DeepService { moneyClient; }",
            ),
            (
                "client/remotes/nested/MoneyClient.js",
                "@HTTPClient\nexport default class MoneyClient {}",
            ),
        ]);
        let registry = ServiceRegistry::discover(root.path()).expect("nested service");
        let service = registry.get("deep").expect("deep");
        assert_eq!(
            service.script_path(),
            Some(
                root.path()
                    .join("client/services/nested/DeepService.js")
                    .as_path()
            )
        );
        assert_eq!(service.injected(), ["moneyClient"]);
        assert!(service.required().is_empty());
    }

    #[test]
    fn assets_barrels_and_abstract_contracts_are_not_registered_as_services() {
        let root = project(&[
            ("client/services/nested/data.json", "{}"),
            (
                "client/services/nested/index.ts",
                "export { Other } from './Other'",
            ),
            (
                "client/services/contracts/BaseService.ts",
                "export abstract class BaseService {}",
            ),
            (
                "client/services/nested/DeepService.js",
                "@Service\nexport default class DeepService {}",
            ),
        ]);
        let registry = ServiceRegistry::discover(root.path()).expect("eligible services only");
        assert_eq!(registry.names(), BTreeSet::from([String::from("deep")]));
    }

    #[test]
    fn an_absent_client_realm_has_no_services() {
        let root = tempfile::tempdir().expect("empty project");
        let registry = ServiceRegistry::discover(root.path()).expect("missing realm");
        assert!(registry.names().is_empty());
    }

    #[test]
    fn nesting_does_not_create_a_second_service_namespace() {
        let error = discover_error(
            &[
                (
                    "client/services/one/DeepService.rs",
                    "#[tac::Service]\npub struct DeepService;",
                ),
                (
                    "client/services/two/DeepService.rs",
                    "#[tac::Service]\npub struct DeepService;",
                ),
            ],
            "duplicate nested service",
        );
        assert!(
            error.to_string().contains("more than one rs file"),
            "{error}"
        );
    }
}
