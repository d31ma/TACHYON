use super::access_check::AccessCheck;
use super::{
    CapturedFile, CapturedSources, CompanionKind, HandlerNode, ProjectDiscovery, RoleSource,
    RouteGraph, SnapshotContext, ViewKind, ViewRoute, ViewSource,
};
use crate::Failure;
use crate::compiler::PortablePath;
use crate::failure::{diagnostic, source_span};
use crate::handler::HandlerSource;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

#[derive(Debug, Default)]
pub(super) struct Discovered {
    views: Vec<(String, ViewSource)>,
    handlers: Vec<(String, HandlerNode)>,
    access: AccessCheck,
    pub(super) diagnostics: Vec<Diagnostic>,
}

impl Discovered {
    const DEFAULT_PATTERN: &str = "/";

    pub(super) fn inspect(&mut self, context: &SnapshotContext<'_>, captured: &CapturedSources) {
        self.add_pages(context, &captured.pages);
        self.add_controllers(context, &captured.controllers);
    }

    pub(super) fn identity_provider(&mut self) -> Option<HandlerSource> {
        let access = std::mem::take(&mut self.access);
        access.finish(&mut self.diagnostics)
    }

    pub(super) fn into_route_graph(self, exports_elements: bool) -> Result<RouteGraph, Failure> {
        let checked = self.checked(exports_elements)?;
        RouteGraph::build(checked.views, checked.handlers)
    }

    fn add_pages(&mut self, context: &SnapshotContext<'_>, files: &[CapturedFile]) {
        for file in files {
            self.add_page(context, file);
        }
    }

    fn add_controllers(&mut self, context: &SnapshotContext<'_>, files: &[CapturedFile]) {
        for file in files {
            self.add_controller(context, file);
        }
    }

    fn add_page(&mut self, context: &SnapshotContext<'_>, file: &CapturedFile) {
        match file.relative.file_name().and_then(OsStr::to_str) {
            None => self.diagnostics.push(Self::not_unicode()),
            Some(name) if name == ViewKind::TAC_FILE_NAME => self.add_view(context, file),
            Some(name) if CompanionKind::from_file_name(name).is_some() => {
                self.diagnostics
                    .push(Self::companion_beside_page(&file.relative, name));
            }
            Some(name)
                if name.starts_with(ViewKind::TAC_STEM)
                    && !RoleSource::is_module_manifest(name) =>
            {
                self.diagnostics
                    .push(Self::companion_without_adapter(&file.relative, name));
            }
            Some(_) => {}
        }
    }

    fn add_view(&mut self, context: &SnapshotContext<'_>, file: &CapturedFile) {
        let route = ViewRoute::of(&file.relative, Path::new(ProjectDiscovery::TAC_ROOT))
            .map_err(|failure| self.diagnostics.extend_from_slice(failure.diagnostics()));
        let view = ViewSource {
            source_path: file.portable(),
            absolute_source_path: context.project_root.join(&file.relative),
            view_kind: ViewKind::Tac,
            bytes: file.bytes.clone(),
        };
        self.views.extend(route.ok().map(|route| (route, view)));
    }

    fn add_controller(&mut self, context: &SnapshotContext<'_>, file: &CapturedFile) {
        if let Some((source, patterns)) = self.routed_controller(context, file) {
            self.access
                .check_routes(&source, &file.portable(), context.project);
            let node = HandlerNode::of(&source);
            self.handlers
                .extend(patterns.into_iter().map(|pattern| (pattern, node.clone())));
        }
    }

    fn routed_controller(
        &mut self,
        context: &SnapshotContext<'_>,
        file: &CapturedFile,
    ) -> Option<(HandlerSource, BTreeSet<String>)> {
        let source = self.controller_source(context, file)?;
        let patterns = Self::patterns(&source);
        self.require_routes(&patterns, file)?;
        Some((source, patterns))
    }

    fn controller_source(
        &mut self,
        context: &SnapshotContext<'_>,
        file: &CapturedFile,
    ) -> Option<HandlerSource> {
        let is_error_handler = std::str::from_utf8(&file.bytes)
            .ok()
            .map_or(Ok(false), |text| {
                crate::annotation_layout::has_error_handler(&file.relative, text)
            })
            .map_err(|failure| self.diagnostics.extend_from_slice(failure.diagnostics()))
            .ok()?;
        (!is_error_handler).then_some(())?;
        (!self.is_identity_provider(context, file)).then_some(())?;
        context
            .discover_handler(&file.relative, file.bytes.clone())
            .map_err(|failure| self.diagnostics.extend_from_slice(failure.diagnostics()))
            .ok()
    }

    fn is_identity_provider(&mut self, context: &SnapshotContext<'_>, file: &CapturedFile) -> bool {
        let found = std::str::from_utf8(&file.bytes)
            .ok()
            .map_or(Ok(false), |text| {
                crate::annotation_layout::has_identity_provider(&file.relative, text)
            })
            .map_err(|failure| self.diagnostics.extend_from_slice(failure.diagnostics()))
            .unwrap_or(false);
        if found {
            self.add_identity_provider(context, file);
        }
        found
    }

    fn add_identity_provider(&mut self, context: &SnapshotContext<'_>, file: &CapturedFile) {
        let source = context
            .discover_handler(&file.relative, file.bytes.clone())
            .map_err(|failure| self.diagnostics.extend_from_slice(failure.diagnostics()))
            .ok();
        if let Some(source) = source {
            self.access.add_identity(source, file.portable());
        }
    }

    fn patterns(source: &HandlerSource) -> BTreeSet<String> {
        source
            .routes()
            .iter()
            .map(|route| String::from(route.pattern.as_deref().unwrap_or(Self::DEFAULT_PATTERN)))
            .collect()
    }

    fn require_routes(&mut self, patterns: &BTreeSet<String>, file: &CapturedFile) -> Option<()> {
        if patterns.is_empty() {
            self.diagnostics.push(Self::no_routes(&file.portable()));
            return None;
        }
        Some(())
    }

    fn checked(self, exports_elements: bool) -> Result<Self, Failure> {
        if !self.diagnostics.is_empty() {
            Err(Failure::new(self.diagnostics))
        } else if self.views.is_empty() && self.handlers.is_empty() && !exports_elements {
            Err(Self::nothing_found())
        } else {
            Ok(self)
        }
    }

    fn not_unicode() -> Diagnostic {
        diagnostic(1005, "Source paths must be valid Unicode.", None, None)
    }

    fn companion_beside_page(relative: &Path, name: &str) -> Diagnostic {
        let path = PortablePath::of(relative);
        diagnostic(
            1309,
            format!("Pages compose components: '{path}' cannot sit beside a page."),
            Some(String::from(
                "A page directory holds only tac.html. Move this file into a component under client/components/<name>/ and place <name /> in the page.",
            )),
            source_span(&path, 0, name.len()),
        )
    }

    fn companion_without_adapter(relative: &Path, name: &str) -> Diagnostic {
        let path = PortablePath::of(relative);
        diagnostic(
            1008,
            format!("Companion source '{path}' has no available adapter."),
            Some(String::from(
                "Supported Tac companions are tac.css, tac.js, tac.ts, tac.rs, tac.swift, tac.kt, and tac.cs.",
            )),
            source_span(&path, 0, name.len()),
        )
    }

    fn no_routes(path: &str) -> Diagnostic {
        diagnostic(
            1310,
            format!("The controller '{path}' has no routes."),
            Some(String::from(
                "Mark the controller class with @RequestRoute(\"/path\") and each route method with @GetRoute, @PostRoute, @PutRoute, @PatchRoute or @DeleteRoute. @GetRoute means the method answers GET requests.",
            )),
            source_span(path, 0, path.len()),
        )
    }

    fn nothing_found() -> Failure {
        Failure::one(diagnostic(
            1002,
            "No Tachyon view or handler source was found.",
            Some(String::from(
                "Add a client/pages/tac.html view, a controller in server/controllers/, or a component to tac.elements in client/manifest.json.",
            )),
            None,
        ))
    }
}
