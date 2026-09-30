use super::site::{RawCacheRule, SiteMetadata, SitemapValues};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NativeApplication {
    pub(super) name: String,
    pub(super) executable_name: String,
    pub(super) application_id: String,
    pub(super) version: String,
    pub(super) entry_route: String,
    pub(super) icons: Vec<(u32, String)>,
    pub(super) window: WindowConfiguration,
    pub(super) native_capabilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct WindowConfiguration {
    #[serde(default = "default_width")]
    pub(super) width: u32,
    #[serde(default = "default_height")]
    pub(super) height: u32,
    #[serde(default)]
    pub(super) min_width: Option<u32>,
    #[serde(default)]
    pub(super) min_height: Option<u32>,
    #[serde(default)]
    pub(super) controls: Vec<String>,
}

const fn default_width() -> u32 {
    1024
}

const fn default_height() -> u32 {
    768
}

impl Default for WindowConfiguration {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            min_width: None,
            min_height: None,
            controls: Vec::new(),
        }
    }
}

impl WindowConfiguration {
    pub(super) const KNOWN: [&'static str; 6] = [
        "close",
        "fullscreen",
        "maximize",
        "minimize",
        "resize",
        "title",
    ];
    const MIN_EDGE: u32 = 64;
    const MAX_EDGE: u32 = 16_384;

    pub(super) fn grants(&self, control: &str) -> bool {
        self.controls.iter().any(|value| value == control)
    }

    fn edge_is_supported(edge: u32) -> bool {
        (Self::MIN_EDGE..=Self::MAX_EDGE).contains(&edge)
    }

    fn has_duplicate_controls(&self) -> bool {
        let unique = self
            .controls
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        unique.len() != self.controls.len()
    }

    fn is_within_bounds(&self) -> bool {
        if !Self::edge_is_supported(self.width) || !Self::edge_is_supported(self.height) {
            return false;
        }
        if self.min_width.is_some_and(|value| value > self.width) {
            return false;
        }
        if self.min_height.is_some_and(|value| value > self.height) {
            return false;
        }
        if self.controls.len() > Self::KNOWN.len() {
            return false;
        }
        !self.has_duplicate_controls()
    }

    fn ensure_known_controls(&self) -> Result<(), Failure> {
        for control in &self.controls {
            if !Self::KNOWN.contains(&control.as_str()) {
                return Err(config_failure(&format!(
                    "'{control}' is not a window control. Grant any of: {}.",
                    Self::KNOWN.join(", ")
                )));
            }
        }
        Ok(())
    }
}

fn manifest_metadata(path: &Path) -> Result<Option<fs::Metadata>, Failure> {
    let manifest_name = WebAppManifest::SOURCE_PATH;
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(config_failure(&format!(
            "Cannot inspect {manifest_name}: {error}"
        ))),
    }
}

pub(super) fn read_manifest(project_root: &Path) -> Result<Option<WebAppManifest>, Failure> {
    let legacy_name = WebAppManifest::LEGACY_NAME;
    let manifest_name = WebAppManifest::SOURCE_PATH;
    if fs::symlink_metadata(project_root.join(legacy_name)).is_ok() {
        return Err(config_failure(&format!(
            "{legacy_name} at the project root moved to {manifest_name}, beside the client it describes (ADR 0034)."
        )));
    }
    let path = project_root.join(manifest_name);
    let Some(metadata) = manifest_metadata(&path)? else {
        return Ok(None);
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(config_failure(&format!(
            "{manifest_name} must be a regular, non-symlinked file."
        )));
    }
    if metadata.len() > WebAppManifest::MAX_BYTES {
        return Err(config_failure(&format!(
            "{manifest_name} exceeds the 1 MiB limit."
        )));
    }
    let source = fs::read_to_string(&path)
        .map_err(|error| config_failure(&format!("Cannot read {manifest_name}: {error}")))?;
    let manifest: WebAppManifest = serde_json::from_str(crate::without_bom(&source))
        .map_err(|error| config_failure(&format!("{manifest_name} is invalid: {error}")))?;
    validate_manifest(&manifest)?;
    Ok(Some(manifest))
}

fn valid_manifest_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.chars().count() <= 64
        && !name.chars().any(char::is_control)
        && !executable_name(name).is_empty()
}

fn valid_icon_source(source: &str) -> bool {
    let relative = source.trim_start_matches('/');
    if relative.is_empty() || source.starts_with("//") {
        return false;
    }
    if relative.contains(['\\', ':', '?', '#', '%']) {
        return false;
    }
    for segment in relative.split('/') {
        if matches!(segment, "" | "." | "..") {
            return false;
        }
    }
    true
}

fn valid_version(version: &str) -> bool {
    if version.is_empty() || version.len() > 64 {
        return false;
    }
    version
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

fn validate_manifest(manifest: &WebAppManifest) -> Result<(), Failure> {
    for name in [&manifest.name, &manifest.short_name].into_iter().flatten() {
        if !valid_manifest_name(name) {
            return Err(config_failure(
                "Manifest names must contain 1 to 64 printable characters and an ASCII letter or digit.",
            ));
        }
    }
    if let Some(route) = &manifest.start_url
        && !valid_route(route)
    {
        return Err(config_failure(
            "Manifest start_url must be a canonical local route.",
        ));
    }
    for icon in &manifest.icons {
        if !valid_icon_source(&icon.src) {
            return Err(config_failure(
                "Manifest icons must identify contained local bundle paths.",
            ));
        }
    }
    if manifest.tachyon.is_some() {
        return Err(config_failure(
            "The manifest member tachyon moved to tac.application. Move id, version and window there.",
        ));
    }
    if let Some(tac) = &manifest.tac
        && (tac.native_capabilities.len() > 1
            || tac
                .native_capabilities
                .iter()
                .any(|grant| grant != "filePicker"))
    {
        return Err(config_failure(
            "Manifest tac.nativeCapabilities accepts filePicker once.",
        ));
    }
    let Some(application) = manifest.application() else {
        return Ok(());
    };
    validate_manifest_application(application)
}

fn validate_manifest_application(application: &ManifestApplication) -> Result<(), Failure> {
    if let Some(id) = &application.id
        && !valid_application_id(id)
    {
        return Err(config_failure(
            "Manifest tac.application.id must be a bounded lowercase reverse-DNS identifier, such as com.example.app.",
        ));
    }
    if let Some(version) = &application.version
        && !valid_version(version)
    {
        return Err(config_failure(
            "Manifest tac.application.version must be a portable version of at most 64 letters, digits, '.', '-' or '+', such as 1.4.0.",
        ));
    }
    let Some(window) = &application.window else {
        return Ok(());
    };
    window.ensure_known_controls()?;
    if !window.is_within_bounds() {
        return Err(config_failure(
            "Manifest tac.application.window sizes must be 64 to 16384, a minimum no larger than its size, and each control listed once.",
        ));
    }
    Ok(())
}

fn icon_edge(icon: &ManifestIcon) -> u32 {
    let Some(sizes) = icon.sizes.as_deref() else {
        return 0;
    };
    let Some(first_size) = sizes.split_whitespace().next() else {
        return 0;
    };
    let Some(width) = first_size.split(['x', 'X']).next() else {
        return 0;
    };
    width.parse::<u32>().unwrap_or(0)
}

fn is_svg(source: &str) -> bool {
    std::path::Path::new(source)
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case("svg"))
}

fn icon_link(icon: &ManifestIcon) -> String {
    use crate::compiler::html_attribute_escape;

    let kind = if is_svg(&icon.src) {
        r#" type="image/svg+xml""#
    } else {
        ""
    };
    let sizes = match icon.sizes.as_deref() {
        Some(value) => format!(r#" sizes="{}""#, html_attribute_escape(value)),
        None => String::new(),
    };
    format!(
        r#"<link rel="icon" href="{}"{kind}{sizes}>"#,
        html_attribute_escape(&icon.src)
    )
}

pub(super) fn manifest_head(manifest: &WebAppManifest) -> String {
    use crate::compiler::html_attribute_escape;
    use std::fmt::Write as _;

    let output_name = WebAppManifest::OUTPUT_NAME;
    let mut head = format!(r#"<link rel="manifest" href="/{output_name}">"#);
    for icon in &manifest.icons {
        head.push_str(&icon_link(icon));
    }
    if let Some(largest) = manifest.largest_icon() {
        let _ = write!(
            head,
            r#"<link rel="apple-touch-icon" href="{}">"#,
            html_attribute_escape(largest)
        );
    }
    head
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebAppManifest {
    name: Option<String>,
    short_name: Option<String>,
    start_url: Option<String>,
    #[serde(default)]
    icons: Vec<ManifestIcon>,
    #[serde(default)]
    tac: Option<TacMember>,
    #[serde(default)]
    tachyon: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ManifestIcon {
    src: String,
    #[serde(default)]
    sizes: Option<String>,
}

impl WebAppManifest {
    const LEGACY_NAME: &str = "manifest.json";
    const MAX_BYTES: u64 = 1_024 * 1_024;
    pub(crate) const SOURCE_PATH: &str = "client/manifest.json";
    pub(crate) const OUTPUT_NAME: &str = "manifest.webmanifest";

    pub(super) fn largest_icon(&self) -> Option<&str> {
        self.icons
            .iter()
            .filter(|icon| !is_svg(&icon.src))
            .max_by_key(|icon| icon_edge(icon))
            .map(|icon| icon.src.as_str())
    }

    pub(super) fn raster_icons(&self) -> Vec<(u32, String)> {
        let mut icons = Vec::new();
        for icon in &self.icons {
            if !is_svg(&icon.src) {
                icons.push((icon_edge(icon), icon.src.clone()));
            }
        }
        icons.sort_by_key(|(edge, _)| std::cmp::Reverse(*edge));
        icons
    }

    fn display_name(&self) -> Option<String> {
        self.name.clone().or_else(|| self.short_name.clone())
    }

    fn application(&self) -> Option<&ManifestApplication> {
        self.tac.as_ref()?.application.as_ref()
    }

    pub(super) fn into_tac(self) -> TacMember {
        self.tac.unwrap_or_default()
    }
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct TacMember {
    #[serde(default)]
    pub(super) origin: Option<String>,
    #[serde(default)]
    application: Option<ManifestApplication>,
    #[serde(default, rename = "nativeCapabilities")]
    pub(super) native_capabilities: Vec<String>,
    #[serde(default)]
    pub(super) scripts: Vec<String>,
    #[serde(default)]
    pub(super) styles: Vec<String>,
    #[serde(default)]
    pub(super) cache: Vec<RawCacheRule>,
    #[serde(default)]
    pub(super) metadata: SiteMetadata,
    #[serde(default)]
    pub(super) sitemap: BTreeMap<String, SitemapValues>,
    #[serde(default, rename = "browserApis")]
    pub(super) browser_apis: Vec<String>,
    #[serde(default)]
    pub(super) elements: Vec<String>,
}

impl TacMember {
    pub(crate) const FRAMEWORK_MEMBERS: [&str; 16] = [
        "origin",
        "application",
        "scripts",
        "styles",
        "cache",
        "metadata",
        "sitemap",
        "cohesion",
        "duplication",
        "dependencies",
        "names",
        "signatures",
        "constants",
        "browserApis",
        "nativeCapabilities",
        "elements",
    ];
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestApplication {
    id: Option<String>,
    version: Option<String>,
    #[serde(default)]
    window: Option<WindowConfiguration>,
}

impl NativeApplication {
    #[cfg(test)]
    pub(super) fn discover(project_root: &Path) -> Result<Self, Failure> {
        Self::discover_from_snapshot(project_root, project_root)
    }

    pub(super) fn discover_from_snapshot(
        project_root: &Path,
        authored_root: &Path,
    ) -> Result<Self, Failure> {
        let manifest = read_manifest(project_root)?;
        Ok(Self::defaults(authored_root).with_manifest(manifest.as_ref()))
    }

    fn with_manifest(mut self, manifest: Option<&WebAppManifest>) -> Self {
        let Some(manifest) = manifest else {
            return self;
        };
        if let Some(name) = manifest.display_name()
            && !name.trim().is_empty()
        {
            self.executable_name = executable_name(&name);
            self.name = name;
        }
        if let Some(start) = &manifest.start_url
            && start.starts_with('/')
        {
            self.entry_route = String::from(start.trim_end_matches('/'));
            if self.entry_route.is_empty() {
                self.entry_route = String::from("/");
            }
        }
        if let Some(application) = manifest.application() {
            self.apply_manifest_application(application);
        }
        self.native_capabilities = manifest
            .tac
            .as_ref()
            .map_or_else(Vec::new, |tac| tac.native_capabilities.clone());
        self.icons = manifest.raster_icons();
        self
    }

    fn apply_manifest_application(&mut self, application: &ManifestApplication) {
        if let Some(id) = &application.id {
            self.application_id.clone_from(id);
        }
        if let Some(version) = &application.version {
            self.version.clone_from(version);
        }
        if let Some(window) = &application.window {
            self.window.clone_from(window);
        }
    }

    pub(super) fn largest_icon(&self) -> Option<&str> {
        self.icons.first().map(|(_, src)| src.as_str())
    }

    pub(super) fn icon_within(&self, edge: u32) -> Option<&str> {
        self.icons
            .iter()
            .find(|(size, _)| *size <= edge)
            .map(|(_, src)| src.as_str())
    }

    fn defaults(project_root: &Path) -> Self {
        let candidate = project_root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("TachyonApp");
        let executable_name = executable_name(candidate);
        let identifier = identifier_segment(&executable_name);
        Self {
            name: executable_name.clone(),
            executable_name,
            application_id: format!("ma.del.tachyon.{identifier}"),
            version: String::from("1.0.0"),
            entry_route: String::from("/"),
            icons: Vec::new(),
            window: WindowConfiguration::default(),
            native_capabilities: Vec::new(),
        }
    }
}

fn executable_name(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(64)
        .collect()
}

fn identifier_segment(value: &str) -> String {
    let mut identifier = String::with_capacity(value.len());
    for byte in value.bytes() {
        let lowercase = byte.to_ascii_lowercase();
        if lowercase.is_ascii_alphanumeric() {
            identifier.push(char::from(lowercase));
        } else {
            identifier.push('-');
        }
    }
    let trimmed = identifier.trim_matches('-');
    if trimmed.is_empty() {
        String::from("app")
    } else {
        String::from(trimmed)
    }
}

fn valid_application_id_segment(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }
    let hyphen_allowed = !segment.starts_with('-') && !segment.ends_with('-');
    segment.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || (byte == b'-' && hyphen_allowed)
    })
}

fn valid_application_id(value: &str) -> bool {
    if value.len() > 255 || value.split('.').count() < 2 {
        return false;
    }
    value.split('.').all(valid_application_id_segment)
}

pub(super) fn valid_route(value: &str) -> bool {
    if !value.starts_with('/') || value.len() > 2_048 {
        return false;
    }
    if value.contains('\\') || value.contains("//") {
        return false;
    }
    !value
        .split('/')
        .any(|segment| matches!(segment, "." | ".."))
}

pub(super) fn config_failure(message: &str) -> Failure {
    let file_name = WebAppManifest::SOURCE_PATH;
    Failure::one(diagnostic(
        1601,
        message,
        Some(String::from(
            "Correct client/manifest.json. Its tac member holds Tachyon's settings: origin, application, scripts, styles, cache, metadata and sitemap.",
        )),
        source_span(file_name, 0, file_name.len()),
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::NativeApplication;
    use std::fs;

    fn write_manifest(root: &std::path::Path, source: &str, sizes: &str) {
        let manifest = serde_json::json!({
            "icons": [{ "src": source, "sizes": sizes }],
        });
        write_manifest_document(root, &manifest);
    }

    fn write_manifest_document(root: &std::path::Path, manifest: &serde_json::Value) {
        fs::create_dir_all(root.join("client")).expect("client");
        fs::write(root.join("client/manifest.json"), manifest.to_string()).expect("manifest");
    }

    fn discovery_error(root: &std::path::Path) -> String {
        NativeApplication::discover(root)
            .expect_err("invalid manifest")
            .to_string()
    }

    type Attributes = std::collections::BTreeMap<String, String>;

    fn start_tags(html: &str) -> Vec<(String, Attributes)> {
        let mut tags = Vec::new();
        for token in html5gum::Tokenizer::new(html) {
            let html5gum::Token::StartTag(tag) = token.expect("HTML token") else {
                continue;
            };
            let name = String::from_utf8(tag.name.to_vec()).expect("tag name");
            let mut attributes = Attributes::new();
            for (attribute_name, value) in tag.attributes {
                attributes.insert(
                    String::from_utf8(attribute_name.to_vec()).expect("attribute name"),
                    String::from_utf8(value.value.to_vec()).expect("attribute value"),
                );
            }
            tags.push((name, attributes));
        }
        tags
    }

    #[test]
    fn manifest_icon_attributes_remain_literal_when_parsed_as_html() {
        for (source, sizes) in [
            (
                "/icons/app.png",
                "16x16\"><script src=\"/shared/injected.js\"></script><link sizes=\"16x16",
            ),
            ("/icons/a&copy;.png", "16x16 &copy;"),
            (
                "/icons/a\"><script src=\"/shared/injected.js\"></script><link href=\"a.png",
                "16x16",
            ),
            ("/icons/<angle>'&copy;\".png", "16x16"),
        ] {
            let root = tempfile::tempdir().expect("project");
            write_manifest(root.path(), source, sizes);
            let manifest = super::read_manifest(root.path())
                .expect("manifest")
                .expect("manifest present");
            let head = super::manifest_head(&manifest);
            let tags = start_tags(&head);
            assert_eq!(
                tags.len(),
                3,
                "metadata must produce only three links: {head}"
            );
            assert!(tags.iter().all(|(name, _)| name == "link"), "{head}");
            assert_eq!(tags[1].1.len(), 3, "no injected icon attributes: {head}");
            assert_eq!(tags[1].1["href"], source);
            assert_eq!(tags[1].1["sizes"], sizes);
            assert_eq!(
                tags[2].1.len(),
                2,
                "no injected Apple icon attributes: {head}"
            );
            assert_eq!(tags[2].1["href"], source);
        }
    }

    #[test]
    fn a_project_without_a_manifest_uses_the_default_identity() {
        let root = tempfile::tempdir().expect("root");
        let defaults = NativeApplication::discover(root.path()).expect("defaults");
        assert!(defaults.application_id.starts_with("ma.del.tachyon."));
        assert_eq!(defaults.version, "1.0.0");
        assert_eq!(defaults.entry_route, "/");
    }

    #[test]
    fn the_manifest_names_and_identifies_the_application() {
        let root = tempfile::tempdir().expect("root");
        write_manifest_document(
            root.path(),
            &serde_json::json!({
                "name": "Catalog App",
                "start_url": "/products/",
                "tac": {
                    "api": { "url": "https://api.example.com" },
                    "application": {
                        "id": "dev.example.catalog",
                        "version": "1.2.3",
                        "window": { "width": 420, "height": 720, "minWidth": 320, "controls": ["close"] },
                    },
                },
            }),
        );
        let configured = NativeApplication::discover(root.path()).expect("configured");
        assert_eq!(configured.name, "Catalog App");
        assert_eq!(configured.executable_name, "CatalogApp");
        assert_eq!(configured.application_id, "dev.example.catalog");
        assert_eq!(configured.version, "1.2.3");
        assert_eq!(configured.entry_route, "/products");
        assert_eq!(configured.window.width, 420);
        assert_eq!(configured.window.height, 720);
        assert_eq!(configured.window.min_width, Some(320));
        assert!(configured.window.grants("close"));
    }

    #[test]
    fn a_manifest_without_tac_application_keeps_the_default_identity() {
        let root = tempfile::tempdir().expect("root");
        write_manifest_document(root.path(), &serde_json::json!({ "name": "Named" }));
        let configured = NativeApplication::discover(root.path()).expect("configured");
        assert_eq!(configured.name, "Named");
        assert!(configured.application_id.starts_with("ma.del.tachyon."));
        assert_eq!(configured.window, super::WindowConfiguration::default());
    }

    #[test]
    fn invalid_manifest_application_values_are_refused_with_a_message() {
        for (application, expected) in [
            (
                serde_json::json!({ "id": "INVALID" }),
                "tac.application.id must be",
            ),
            (
                serde_json::json!({ "id": "single" }),
                "tac.application.id must be",
            ),
            (
                serde_json::json!({ "version": "bad version" }),
                "tac.application.version must be",
            ),
            (
                serde_json::json!({ "version": "" }),
                "tac.application.version must be",
            ),
            (
                serde_json::json!({ "window": { "width": 10 } }),
                "tac.application.window sizes",
            ),
            (
                serde_json::json!({ "window": { "width": 400, "minWidth": 500 } }),
                "tac.application.window sizes",
            ),
            (
                serde_json::json!({ "window": { "controls": ["close", "close"] } }),
                "tac.application.window sizes",
            ),
            (
                serde_json::json!({ "window": { "controls": ["teleport"] } }),
                "'teleport' is not a window control",
            ),
            (
                serde_json::json!({ "window": { "minwidth": 300 } }),
                "unknown field `minwidth`",
            ),
            (serde_json::json!({ "name": "App" }), "unknown field `name`"),
            (
                serde_json::json!({ "entryRoute": "/" }),
                "unknown field `entryRoute`",
            ),
        ] {
            let root = tempfile::tempdir().expect("root");
            write_manifest_document(
                root.path(),
                &serde_json::json!({ "tac": { "application": application } }),
            );
            let text = discovery_error(root.path());
            assert!(text.contains("TY1601"), "{text}");
            assert!(text.contains(expected), "{application}: {text}");
        }
    }

    #[test]
    fn the_old_tachyon_manifest_member_is_refused() {
        let root = tempfile::tempdir().expect("root");
        write_manifest_document(
            root.path(),
            &serde_json::json!({ "tachyon": { "id": "dev.example.app" } }),
        );
        let text = discovery_error(root.path());
        assert!(text.contains("moved to tac.application"), "{text}");
    }
}
