use super::WebCompiler;
use super::page_head::PageHead;
use crate::Failure;
use crate::failure::diagnostic;
use crate::routing::MountPath;
use crate::sorted_json::SortedJson;
use crate::template::{ClientViewRenderer, ComponentDefinition, ComponentRegistry};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;

pub(super) struct ShellLayout {
    plan: Option<Value>,
    tag: String,
    text: String,
    style_link: &'static str,
    services: BTreeSet<String>,
}

impl ShellLayout {
    const CODE: u16 = 1601;
    const OUTLET: &'static str = "{\"k\":\"outlet\"}";
    const VIEW_MARK: &'static str = "id=\"tachyon-view\"";
    const HEAD_END: &'static str = "</head>";
    const FILE_PATH: &'static str = ".tachyon/shell.json";
    const SCRIPT_PATH: &'static str = ".tachyon/tac-shell.js";
    const PRELOAD: &'static str =
        r#"<link rel="modulepreload" href="/.tachyon/tac-shell.js" data-tachyon-runtime>"#;
    const COMPONENT_STYLES: &'static str = "/.tachyon/components.css";
    const HELP: &'static str = "tac.layout names the shell's layout component, such as \"app-shell\". Its template holds exactly one <outlet />, where each page renders (ADR NNNN).";

    pub(super) fn none() -> Self {
        Self {
            plan: None,
            tag: String::new(),
            text: String::new(),
            style_link: "",
            services: BTreeSet::new(),
        }
    }

    pub(super) fn build(
        layout: Option<&str>,
        mount: &MountPath,
        renderer: ClientViewRenderer<'_>,
        components: &ComponentRegistry,
    ) -> Result<Self, Failure> {
        layout.map_or_else(
            || Ok(Self::none()),
            |name| Self::built(name, mount, renderer, components),
        )
    }

    pub(super) fn with_styles(mut self, has_styles: bool) -> Result<Self, Failure> {
        self.style_link = ["", WebCompiler::COMPONENT_STYLE_LINK][usize::from(has_styles)];
        self.plan = self.plan.map(|plan| Self::styled(plan, has_styles));
        self.text = self
            .plan
            .as_ref()
            .map(Self::serialized)
            .transpose()?
            .unwrap_or_default();
        self.tag = Self::script(&self.text);
        Ok(self)
    }

    pub(super) fn files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        self.plan
            .as_ref()
            .map(|_| self.written_files())
            .unwrap_or_default()
    }

    fn written_files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![
            (
                PathBuf::from(Self::FILE_PATH),
                self.text.clone().into_bytes(),
            ),
            (
                PathBuf::from(Self::SCRIPT_PATH),
                ClientViewRenderer::shipped_shell().as_bytes().to_vec(),
            ),
        ]
    }

    pub(super) fn tag(&self) -> &str {
        &self.tag
    }

    pub(super) fn services(&self) -> &BTreeSet<String> {
        &self.services
    }

    pub(super) fn check_page(route: &str, html: &str) -> Result<(), Failure> {
        Self::refuse_page_outlet(route, html.contains(Self::OUTLET))
    }

    fn refuse_page_outlet(route: &str, has_outlet: bool) -> Result<(), Failure> {
        if !has_outlet {
            return Ok(());
        }
        Err(Self::failure(&format!(
            "The page {route} holds <outlet />. Only the shell's layout component (tac.layout) holds the outlet."
        )))
    }

    pub(super) fn app_documents(&self, files: Vec<(PathBuf, Vec<u8>)>) -> Vec<(PathBuf, Vec<u8>)> {
        files
            .into_iter()
            .map(|file| self.app_document(file))
            .collect()
    }

    fn app_document(&self, (path, bytes): (PathBuf, Vec<u8>)) -> (PathBuf, Vec<u8>) {
        let page = std::str::from_utf8(&bytes).is_ok_and(|text| text.contains(Self::VIEW_MARK));
        if !page {
            return (path, bytes);
        }
        let text = String::from_utf8_lossy(&bytes);
        let added = format!("{}{}", self.style_link, self.tag);
        let html = PageHead::inject_before(&text, Self::HEAD_END, &added);
        (path, html.into_bytes())
    }

    fn built(
        name: &str,
        mount: &MountPath,
        renderer: ClientViewRenderer<'_>,
        components: &ComponentRegistry,
    ) -> Result<Self, Failure> {
        Self::check_shell(mount)?;
        let component = Self::component(components, name)?;
        let mut services = BTreeSet::new();
        let element = renderer.element(name, component, &mut services)?;
        let plan = json!({
            "node": element["node"],
            "services": element["services"],
            "saved": element.get("saved"),
            "tachyon": env!("CARGO_PKG_VERSION"),
        });
        Self::check_outlets(name, &Self::serialized(&plan)?)?;
        Ok(Self {
            plan: Some(plan),
            tag: String::new(),
            text: String::new(),
            style_link: "",
            services,
        })
    }

    fn styled(mut plan: Value, has_styles: bool) -> Value {
        let styles: Vec<&str> = has_styles
            .then_some(Self::COMPONENT_STYLES)
            .into_iter()
            .collect();
        plan["styles"] = json!(styles);
        plan
    }

    fn script(text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        format!(
            r#"{}<script id="tachyon-shell" type="application/json">{text}</script>"#,
            Self::PRELOAD
        )
    }

    fn check_shell(mount: &MountPath) -> Result<(), Failure> {
        if mount.is_root() {
            return Ok(());
        }
        Err(Self::failure(
            "A page app with tac.mount cannot declare tac.layout. Only the shell, the app at /, has the layout.",
        ))
    }

    fn component<'a>(
        components: &'a ComponentRegistry,
        name: &str,
    ) -> Result<&'a ComponentDefinition, Failure> {
        components.get(name).ok_or_else(|| {
            Self::failure(&format!(
                "tac.layout names <{name}>, which is not a component of this project."
            ))
        })
    }

    fn serialized(plan: &Value) -> Result<String, Failure> {
        let text = serde_json::to_string(&SortedJson::value(plan))
            .map_err(|error| Self::failure(&error.to_string()))?;
        Ok(text.replace('<', "\\u003c"))
    }

    fn check_outlets(name: &str, text: &str) -> Result<(), Failure> {
        if text.matches(Self::OUTLET).count() == 1 {
            return Ok(());
        }
        Err(Self::failure(&format!(
            "The layout component <{name}> must hold exactly one <outlet />, where each page renders."
        )))
    }

    fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(String::from(Self::HELP)),
            None,
        ))
    }
}
