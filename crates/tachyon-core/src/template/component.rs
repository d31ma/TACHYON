use super::model::{AttributeValue, TemplateAttribute};
use super::{TemplateFrontend, TemplateNode, TemplateNodeKind, TemplateProgram};
use crate::Failure;
use crate::compiler::{hex_digest, portable_path};
use crate::failure::{diagnostic, source_span};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Component as PathComponent, Path, PathBuf};
use tachyon_diagnostics::Diagnostic;

#[derive(Clone, Debug, Default)]
pub(crate) struct ComponentRegistry {
    components: BTreeMap<String, ComponentDefinition>,
    digest: String,
}

impl ComponentRegistry {
    const COMPONENT_ROOT: &'static str = "client/components";
    const MAX_COMPONENTS: usize = 1_024;
    pub(crate) const SCOPE_ATTRIBUTE: &'static str = "data-tac-scope";

    pub(crate) fn discover(project_root: &Path) -> Result<Self, Failure> {
        let Some(root) = regular_component_root(project_root)? else {
            return Ok(Self::default());
        };
        let mut discovered = BTreeMap::<String, ComponentFiles>::new();
        let mut diagnostics = Vec::new();
        visit_components(
            project_root,
            &root,
            &root,
            &mut discovered,
            &mut diagnostics,
        );
        if discovered.len() > Self::MAX_COMPONENTS {
            diagnostics.push(component_diagnostic(
                1401,
                Self::COMPONENT_ROOT,
                0,
                Self::COMPONENT_ROOT.len(),
                "Component registry exceeds the limit of 1,024.",
                "Split the application or remove unused components.",
            ));
        }
        let mut hasher = Sha256::new();
        let components = define_components(project_root, discovered, &mut hasher, &mut diagnostics);
        detect_cycles(&components, &mut diagnostics);
        sort_diagnostics(&mut diagnostics);
        if !diagnostics.is_empty() {
            return Err(Failure::new(diagnostics));
        }
        Ok(Self {
            components,
            digest: hex_digest(hasher.finalize()),
        })
    }

    pub(crate) fn names(&self) -> BTreeSet<String> {
        self.components.keys().cloned().collect()
    }

    pub(crate) fn get(&self, name: &str) -> Option<&ComponentDefinition> {
        self.components.get(name)
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }
}

fn regular_component_root(project_root: &Path) -> Result<Option<PathBuf>, Failure> {
    let component_root = ComponentRegistry::COMPONENT_ROOT;
    let root = project_root.join(component_root);
    let metadata = match fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Failure::one(component_diagnostic(
                1401,
                component_root,
                0,
                0,
                &format!("Cannot inspect component root: {error}"),
                "Use a readable regular component directory.",
            )));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Failure::one(component_diagnostic(
            1401,
            component_root,
            0,
            component_root.len(),
            "Component root must be a regular directory.",
            "Replace the source root with a non-symlinked directory.",
        )));
    }
    Ok(Some(root))
}

#[derive(Clone, Debug)]
pub(crate) struct ComponentDefinition {
    program: TemplateProgram,
    style_path: Option<PathBuf>,
}

impl ComponentDefinition {
    pub(crate) fn program(&self) -> &TemplateProgram {
        &self.program
    }

    pub(crate) fn style_path(&self) -> Option<&Path> {
        self.style_path.as_deref()
    }
}

#[derive(Clone, Debug, Default)]
struct ComponentFiles {
    template: Option<PathBuf>,
    style: Option<PathBuf>,
}

fn visit_components(
    project_root: &Path,
    root: &Path,
    directory: &Path,
    components: &mut BTreeMap<String, ComponentFiles>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(entries) = sorted_entries(project_root, directory, diagnostics) else {
        return;
    };
    for entry in entries {
        visit_entry(project_root, root, &entry.path(), components, diagnostics);
    }
}

fn sorted_entries(
    project_root: &Path,
    directory: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<fs::DirEntry>> {
    let listing = fs::read_dir(directory).and_then(Iterator::collect::<Result<Vec<_>, _>>);
    let mut entries = match listing {
        Ok(entries) => entries,
        Err(error) => {
            let relative = portable_path(directory.strip_prefix(project_root).unwrap_or(directory));
            diagnostics.push(component_diagnostic(
                1401,
                &relative,
                0,
                relative.len(),
                &format!("Cannot enumerate component directory: {error}"),
                "Check the directory permissions.",
            ));
            return None;
        }
    };
    entries.sort_by_key(fs::DirEntry::file_name);
    Some(entries)
}

fn visit_entry(
    project_root: &Path,
    root: &Path,
    path: &Path,
    components: &mut BTreeMap<String, ComponentFiles>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let relative = portable_path(path.strip_prefix(project_root).unwrap_or(path));
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            diagnostics.push(component_diagnostic(
                1401,
                &relative,
                0,
                0,
                &format!("Cannot inspect component source: {error}"),
                "Use readable regular component sources.",
            ));
            return;
        }
    };
    if metadata.file_type().is_symlink() {
        diagnostics.push(component_diagnostic(
            1401,
            &relative,
            0,
            relative.len(),
            "Symlinked component sources are not allowed.",
            "Replace the symlink with a regular file or directory.",
        ));
        return;
    }
    if metadata.is_dir() {
        visit_components(project_root, root, path, components, diagnostics);
        return;
    }
    if metadata.is_file() {
        inspect_component_file(root, path, components, diagnostics);
    }
}

fn inspect_component_file(
    root: &Path,
    path: &Path,
    components: &mut BTreeMap<String, ComponentFiles>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        diagnostics.push(component_diagnostic(
            1401,
            ComponentRegistry::COMPONENT_ROOT,
            0,
            0,
            "Component source paths must be valid Unicode.",
            "Rename the source using portable Unicode.",
        ));
        return;
    };
    if !matches!(name, "tac.html" | "tac.css") {
        report_file_beside_view(root, path, name, diagnostics);
        return;
    }
    let Some(parent) = path.parent() else {
        return;
    };
    let Some(component_name) = component_name(root, parent) else {
        let relative = portable_path(path.strip_prefix(root).unwrap_or(path));
        diagnostics.push(component_diagnostic(
            1401,
            &relative,
            0,
            relative.len(),
            "Component path must name a tag that is not a standard HTML element.",
            "Rename the directory: client/components/date-picker/tac.html names \
             <date-picker>, and client/components/product/card/tac.html names \
             <product-card>.",
        ));
        return;
    };
    let files = components.entry(component_name).or_default();
    if name == "tac.html" {
        files.template = Some(path.to_path_buf());
    } else {
        files.style = Some(path.to_path_buf());
    }
}

fn report_file_beside_view(
    root: &Path,
    path: &Path,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if name == "tac.modules.json" {
        return;
    }
    let relative = portable_path(path.strip_prefix(root).unwrap_or(path));
    let is_island = matches!(name, "tachyon-island.js" | "tachyon-island.ts");
    if crate::project::companion_kind(name).is_some() || is_island {
        diagnostics.push(component_diagnostic(
            1310,
            &relative,
            0,
            relative.len(),
            &format!("Components use services: '{relative}' cannot sit beside a component view."),
            "A component directory holds tac.html and tac.css. Move this file to \
             client/services/<Name>Service.<ext>, annotate the class @Service(), and \
             address it from the view as {<name>.field} (ADR 0030).",
        ));
        return;
    }
    if name.starts_with("tac.") || name.starts_with("tachyon-wasm.") {
        diagnostics.push(component_diagnostic(
            1401,
            &relative,
            0,
            relative.len(),
            "Component companion is not a supported file.",
            "A component directory holds tac.html and tac.css; behaviour lives in \
             client/services (ADR 0030). Browser WASM companions are retired.",
        ));
    }
}

fn define_components(
    project_root: &Path,
    discovered: BTreeMap<String, ComponentFiles>,
    hasher: &mut Sha256,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, ComponentDefinition> {
    let names = discovered.keys().cloned().collect::<BTreeSet<_>>();
    let mut components = BTreeMap::new();
    for (name, mut files) in discovered {
        let Some(template) = files.template.take() else {
            report_orphan_style(project_root, &name, &files, diagnostics);
            continue;
        };
        let relative_template =
            portable_path(template.strip_prefix(project_root).unwrap_or(&template));
        let program = match TemplateFrontend::compile_file(&template, &relative_template, &names) {
            Ok(program) => program,
            Err(failure) => {
                diagnostics.extend_from_slice(failure.diagnostics());
                continue;
            }
        };
        hash_file(hasher, project_root, &template, diagnostics);
        let definition = define_component(program, &name, files, project_root, hasher, diagnostics);
        components.insert(name, definition);
    }
    components
}

fn report_orphan_style(
    project_root: &Path,
    name: &str,
    files: &ComponentFiles,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(style) = &files.style else {
        return;
    };
    diagnostics.push(component_diagnostic(
        1401,
        &portable_path(style.strip_prefix(project_root).unwrap_or(style)),
        0,
        0,
        &format!("Component '{name}' has a stylesheet but no tac.html."),
        "Add the component template or remove the orphan stylesheet.",
    ));
}

fn define_component(
    mut program: TemplateProgram,
    name: &str,
    files: ComponentFiles,
    project_root: &Path,
    hasher: &mut Sha256,
    diagnostics: &mut Vec<Diagnostic>,
) -> ComponentDefinition {
    if let Some(style) = &files.style {
        hash_file(hasher, project_root, style, diagnostics);
        mark_scope_roots(&mut program.nodes, name);
    }
    ComponentDefinition {
        program,
        style_path: files.style,
    }
}

fn mark_scope_roots(nodes: &mut [TemplateNode], component: &str) {
    for node in nodes {
        match &mut node.kind {
            TemplateNodeKind::Element { attributes, .. } => {
                attributes.insert(
                    String::from(ComponentRegistry::SCOPE_ATTRIBUTE),
                    TemplateAttribute {
                        value: AttributeValue::Static(String::from(component)),
                        range: node.range,
                    },
                );
            }
            TemplateNodeKind::Conditional { children, .. }
            | TemplateNodeKind::Iteration { children, .. }
            | TemplateNodeKind::CountedIteration { children, .. } => {
                mark_scope_roots(children, component);
            }
            _ => {}
        }
    }
}

fn component_name(root: &Path, directory: &Path) -> Option<String> {
    let relative = directory.strip_prefix(root).ok()?;
    let mut segments = Vec::new();
    for component in relative.components() {
        if let PathComponent::Normal(value) = component
            && let Some(segment) = value.to_str()
        {
            segments.push(segment);
        }
    }
    if segments.is_empty() || !segments.iter().all(|segment| valid_segment(segment)) {
        return None;
    }
    let name = segments.join("-");
    if crate::html::is_standard_html_tag(&name) {
        return None;
    }
    Some(name)
}

fn valid_segment(segment: &str) -> bool {
    let starts_with_lowercase = segment
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase());
    let has_only_tag_characters = segment
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    starts_with_lowercase && has_only_tag_characters
}

fn detect_cycles(
    components: &BTreeMap<String, ComponentDefinition>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut complete = BTreeSet::new();
    for name in components.keys() {
        let mut active = Vec::new();
        visit_cycle(name, components, &mut active, &mut complete, diagnostics);
    }
}

fn visit_cycle(
    name: &str,
    components: &BTreeMap<String, ComponentDefinition>,
    active: &mut Vec<String>,
    complete: &mut BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if complete.contains(name) {
        return;
    }
    if let Some(position) = active.iter().position(|active_name| active_name == name) {
        diagnostics.push(cycle_diagnostic(name, &active[position..], components));
        return;
    }
    active.push(String::from(name));
    if let Some(component) = components.get(name) {
        let mut references = BTreeSet::new();
        collect_component_references(&component.program.nodes, &mut references);
        for reference in references {
            visit_cycle(&reference, components, active, complete, diagnostics);
        }
    }
    active.pop();
    complete.insert(String::from(name));
}

fn cycle_diagnostic(
    name: &str,
    active_cycle: &[String],
    components: &BTreeMap<String, ComponentDefinition>,
) -> Diagnostic {
    let mut cycle = active_cycle.to_vec();
    cycle.push(String::from(name));
    let source = match components.get(name) {
        Some(component) => component.program.source_path.as_str(),
        None => ComponentRegistry::COMPONENT_ROOT,
    };
    component_diagnostic(
        1403,
        source,
        0,
        0,
        &format!("Tac component cycle detected: {}.", cycle.join(" -> ")),
        "Remove one component invocation from the cycle.",
    )
}

fn collect_component_references(nodes: &[TemplateNode], references: &mut BTreeSet<String>) {
    for node in nodes {
        if let TemplateNodeKind::Component { name, .. } = &node.kind {
            references.insert(name.clone());
        }
        if let Some(children) = node.kind.children() {
            collect_component_references(children, references);
        }
    }
}

fn hash_file(
    hasher: &mut Sha256,
    project_root: &Path,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let relative = portable_path(path.strip_prefix(project_root).unwrap_or(path));
    match fs::read(path) {
        Ok(bytes) => {
            hasher.update(relative.as_bytes());
            hasher.update([0]);
            hasher.update(bytes);
            hasher.update([0]);
        }
        Err(error) => diagnostics.push(component_diagnostic(
            1401,
            &relative,
            0,
            0,
            &format!("Cannot hash component source: {error}"),
            "Check the source file permissions.",
        )),
    }
}

fn component_diagnostic(
    number: u16,
    source_path: &str,
    start: usize,
    end: usize,
    message: &str,
    help: &str,
) -> Diagnostic {
    diagnostic(
        number,
        message,
        Some(String::from(help)),
        source_span(source_path, start, end),
    )
}

fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|left, right| {
        diagnostic_position(left)
            .cmp(&diagnostic_position(right))
            .then_with(|| left.code.cmp(&right.code))
    });
}

fn diagnostic_position(diagnostic: &Diagnostic) -> Option<(&String, u64, u64)> {
    let span = diagnostic.spans.first()?;
    Some((&span.file, span.start, span.end))
}

#[cfg(all(test, not(coverage)))]
mod tests {
    #![allow(clippy::expect_used)]

    use super::ComponentRegistry;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn project() -> TempDir {
        tempfile::tempdir().unwrap_or_else(|_| unreachable!())
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap_or_else(|| unreachable!()))
            .unwrap_or_else(|_| unreachable!());
        fs::write(path, contents).unwrap_or_else(|_| unreachable!());
    }

    #[test]
    fn a_component_directory_names_its_tag_at_any_depth() {
        let root = project();
        for relative in [
            "client/components/clicker/tac.html",
            "client/components/date-picker/tac.html",
            "client/components/product/card/tac.html",
        ] {
            write(&root.path().join(relative), "<div>x</div>");
        }

        let registry = ComponentRegistry::discover(root.path()).expect("registry");
        let names = registry.names();
        assert!(names.contains("clicker"), "{names:?}");
        assert!(names.contains("date-picker"), "{names:?}");
        assert!(names.contains("product-card"), "{names:?}");
    }

    #[test]
    fn a_component_may_not_shadow_a_standard_html_element() {
        let root = project();
        write(
            &root.path().join("client/components/section/tac.html"),
            "<div>x</div>",
        );

        let error = ComponentRegistry::discover(root.path()).expect_err("shadowed element");
        assert!(error.to_string().contains("TY1401"), "{error}");
        assert!(
            error.to_string().contains("standard HTML element"),
            "{error}"
        );
    }

    #[test]
    fn registry_discovers_components_styles_and_a_stable_digest() {
        let root = project();
        let component = root.path().join("client/components/product/card");
        write(&component.join("tac.html"), "<p>{label}</p>");
        write(&component.join("tac.css"), "p { color: red }");
        let first = ComponentRegistry::discover(root.path()).unwrap_or_else(|_| unreachable!());
        let second = ComponentRegistry::discover(root.path()).unwrap_or_else(|_| unreachable!());
        assert_eq!(first.digest(), second.digest());
        let selected = first
            .get("product-card")
            .and_then(super::ComponentDefinition::style_path)
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str);
        assert_eq!(selected, Some("tac.css"));

        write(&component.join("tac.css"), "p { color: blue }");
        let third = ComponentRegistry::discover(root.path()).unwrap_or_else(|_| unreachable!());
        assert_ne!(first.digest(), third.digest());
    }

    #[test]
    fn a_browser_script_beside_a_component_view_fails_ty1310() {
        let root = project();
        let component = root.path().join("client/components/product/card");
        write(&component.join("tac.html"), "<p>{card.label}</p>");
        write(
            &component.join("tac.js"),
            "export default class Card { label = 'x' }",
        );

        let failure = ComponentRegistry::discover(root.path()).expect_err("script beside view");
        let text = failure.to_string();
        assert!(text.contains("TY1310"), "{text}");
        assert!(text.contains("Components use services"), "{text}");
        assert!(text.contains("product/card/tac.js"), "{text}");
        assert!(
            text.contains("client/services/<Name>Service.<ext>"),
            "{text}"
        );
        assert!(text.contains("ADR 0030"), "{text}");
    }

    #[test]
    fn a_typescript_script_beside_a_component_view_fails_ty1310() {
        let root = project();
        let component = root.path().join("client/components/typed/card");
        write(&component.join("tac.html"), "<p>{card.label}</p>");
        write(
            &component.join("tac.ts"),
            "export default class Card { label: string = 'x' }",
        );

        let failure = ComponentRegistry::discover(root.path()).expect_err("script beside view");
        assert!(failure.to_string().contains("TY1310"), "{failure}");
        assert!(
            failure.to_string().contains("typed/card/tac.ts"),
            "{failure}"
        );
    }

    #[test]
    fn native_companions_beside_a_component_view_fail_ty1310() {
        let root = project();
        let component = root.path().join("client/components/owned/card");
        write(&component.join("tac.html"), "<p>{card.count}</p>");
        write(
            &component.join("tac.swift"),
            "final class Companion { var count = 1 }",
        );
        write(
            &component.join("tac.rs"),
            "#[derive(Default)] struct Companion { count: i64 }",
        );
        write(
            &component.join("tac.kt"),
            "class Companion { var count = 1 }",
        );
        write(
            &component.join("tac.cs"),
            "public class Companion { public int Count = 1; }",
        );

        let failure = ComponentRegistry::discover(root.path()).expect_err("native beside view");
        let text = failure.to_string();
        let ty1310 = failure
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code.number() == 1310)
            .count();
        assert_eq!(ty1310, 4, "{text}");
        for name in ["tac.swift", "tac.rs", "tac.kt", "tac.cs"] {
            assert!(text.contains(&format!("owned/card/{name}")), "{text}");
        }
    }

    #[test]
    fn island_sidecars_beside_a_component_view_fail_ty1310() {
        let root = project();
        let component = root.path().join("client/components/dual/island");
        write(&component.join("tac.html"), "<p>{label}</p>");
        write(
            &component.join("tachyon-island.js"),
            "export default class RewriteIsland {}",
        );

        let failure = ComponentRegistry::discover(root.path()).expect_err("island sidecar");
        assert!(failure.to_string().contains("TY1310"), "{failure}");
        assert!(
            failure
                .to_string()
                .contains("dual/island/tachyon-island.js"),
            "{failure}"
        );
    }

    #[test]
    fn wasm_sidecars_beside_a_component_view_are_unsupported_files() {
        let root = project();
        let component = root.path().join("client/components/language/rust");
        write(&component.join("tac.html"), "<p>{count}</p>");
        write(
            &component.join("tachyon-wasm.rs"),
            "#[no_mangle] pub extern \"C\" fn tac_alloc(_: i32) -> i32 { 0 }",
        );

        let failure =
            ComponentRegistry::discover(root.path()).expect_err("obsolete wasm companion");
        let text = failure.to_string();
        assert!(text.contains("TY1401"), "{text}");
        assert!(text.contains("tachyon-wasm.rs"), "{text}");
        assert!(
            text.contains("Component companion is not a supported file."),
            "{text}"
        );
        assert!(text.contains("ADR 0030"), "{text}");
    }

    #[test]
    fn a_style_is_kept_beside_its_view() {
        let root = project();
        let component = root.path().join("client/components/styled/card");
        write(&component.join("tac.html"), "<p>{label}</p>");
        write(&component.join("tac.css"), "p { margin: 0 }");

        let registry = ComponentRegistry::discover(root.path()).expect("registry");
        let card = registry
            .get("styled-card")
            .unwrap_or_else(|| unreachable!());
        assert!(card.style_path().is_some());
    }

    #[test]
    fn invalid_shapes_orphans_and_cycles_fail_closed() {
        let root = project();
        let components = root.path().join("client/components");
        write(
            &components.join("one/tac.html"),
            "<p>invalid single segment</p>",
        );
        write(
            &components.join("cycle/one/tac.html"),
            "<cycle-two></cycle-two>",
        );
        write(
            &components.join("cycle/two/tac.html"),
            "<cycle-one></cycle-one>",
        );
        write(&components.join("orphan/style/tac.css"), "p { margin: 0 }");
        let error = ComponentRegistry::discover(root.path()).expect_err("invalid components");
        assert!(error.to_string().contains("TY1401"), "{error}");
        assert!(error.to_string().contains("TY1403"), "{error}");
    }

    #[test]
    fn a_component_root_that_is_a_file_fails_closed() {
        let root = project();
        write(&root.path().join("client/components"), "not a directory");
        assert!(ComponentRegistry::discover(root.path()).is_err());
    }

    #[test]
    fn a_malformed_template_and_an_unsupported_companion_fail_closed() {
        let root = project();
        let component = root.path().join("client/components/bad/template");
        write(&component.join("tac.html"), "<main>");
        write(&component.join("tac.py"), "x = 1");
        let error = ComponentRegistry::discover(root.path()).expect_err("malformed");
        assert!(error.to_string().contains("TY1301"));
        assert!(error.to_string().contains("TY1401"));
    }

    #[test]
    fn a_binary_script_beside_a_view_fails_closed() {
        let root = project();
        let component = root.path().join("client/components/bad/script");
        write(&component.join("tac.html"), "<p>safe</p>");
        fs::write(component.join("tac.js"), b"bad\0script").unwrap_or_else(|_| unreachable!());
        let error = ComponentRegistry::discover(root.path()).expect_err("script beside view");
        assert!(error.to_string().contains("TY1310"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_component_root_fails_closed() {
        use std::os::unix::fs::symlink;
        let root = project();
        let outside = root.path().join("outside");
        fs::create_dir_all(&outside).unwrap_or_else(|_| unreachable!());
        fs::create_dir_all(root.path().join("client")).unwrap_or_else(|_| unreachable!());
        symlink(&outside, root.path().join("client/components")).unwrap_or_else(|_| unreachable!());
        assert!(ComponentRegistry::discover(root.path()).is_err());
    }
}
