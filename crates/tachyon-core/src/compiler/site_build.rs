use super::build_stage::BuildStage;
use super::build_state::BuildState;
use super::component_styles::ComponentStyles;
use super::content_digest::ContentDigest;
use super::exported_elements::ExportedElements;
use super::output_path::OutputPath;
use super::page_composition::PageComposition;
use super::page_head::PageHead;
use super::page_state::PageState;
use super::portable_path::PortablePath;
use super::precompression::CompressedFiles;
use super::route_build_state::RouteBuildState;
use super::service_checks::ServiceChecks;
use super::service_script::ServiceScript;
use super::service_worker::ServiceWorker;
use super::shared_assets::SharedAssets;
use super::tool_build::ToolBuild;
use super::{BuildOptions, BuildProfile, BuildResult, WebCompiler};
use crate::failure::diagnostic;
use crate::tac_web::{BuildJobs, BuiltServices};
use crate::template::{
    ClientViewRenderer, ComponentRegistry, ServiceDefinition, ServiceRegistry, TemplateFrontend,
    TemplateProgram, client_route_context,
};
use crate::{Failure, Project, RouteNode};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tachyon_diagnostics::Diagnostic;

pub(super) struct SiteBuild<'a> {
    project: &'a Project,
    pub(super) snapshot_root: &'a Path,
    pub(super) output_directory: PathBuf,
    components: ComponentRegistry,
    services: ServiceRegistry,
    shared_data: BTreeSet<String>,
    component_styles: String,
    build_config_digest: Option<String>,
    tool_build: String,
    files: Vec<(PathBuf, Vec<u8>)>,
    next_state: BuildState,
    compiled_routes: usize,
    reused_routes: usize,
    profile: BuildProfile,
    compress: bool,
    phase_started: Instant,
    timings: Vec<(String, Duration)>,
    service_timings: Vec<(String, Duration)>,
    service_notes: Vec<String>,
}

impl<'a> SiteBuild<'a> {
    pub(super) const SERVICES_PHASE: &'static str = "services";

    pub(super) fn finish_phase(&mut self, phase: &str) {
        self.timings
            .push((String::from(phase), self.phase_started.elapsed()));
        self.phase_started = Instant::now();
    }

    pub(super) fn discover(
        project: &'a Project,
        options: &BuildOptions,
        native_target: Option<tachyon_contracts::NativeTarget>,
    ) -> Result<Self, Failure> {
        let phase_started = Instant::now();
        let output_directory = OutputPath::resolve(project.root(), &options.output_directory)?;
        let snapshot_root = project.snapshot_root();
        let components = ComponentRegistry::discover(snapshot_root)?;
        let services = ServiceRegistry::discover(snapshot_root)?.for_target(native_target);
        ServiceChecks::validate_all(snapshot_root, &services)?;
        let shared_data = SharedAssets::data_names(snapshot_root);
        let component_styles = ComponentStyles::collect(&components)?;
        let build_config_digest = BuildStage::config_digest(snapshot_root)?;
        Ok(Self {
            project,
            snapshot_root,
            output_directory,
            components,
            services,
            shared_data,
            component_styles,
            build_config_digest,
            tool_build: ToolBuild::id(std::env::current_exe().ok().as_deref()),
            files: Vec::new(),
            next_state: BuildState::default(),
            compiled_routes: 0,
            reused_routes: 0,
            profile: options.profile,
            compress: [
                options.profile == BuildProfile::Release,
                native_target.is_none(),
            ] == [true, true],
            phase_started,
            timings: Vec::new(),
            service_timings: Vec::new(),
            service_notes: Vec::new(),
        })
    }

    pub(super) fn compile_route_programs(
        &self,
    ) -> Result<BTreeMap<String, TemplateProgram>, Failure> {
        let mut programs = BTreeMap::new();
        let mut diagnostics = Vec::new();
        self.compile_each_route(&mut programs, &mut diagnostics)?;
        PageComposition::sort_diagnostics(&mut diagnostics);
        Self::programs_or_failure(programs, diagnostics)
    }

    pub(super) fn add_route_files(
        &mut self,
        programs: &BTreeMap<String, TemplateProgram>,
        previous: Option<&BuildState>,
        site: &crate::native::SiteSettings,
    ) -> Result<(), Failure> {
        let dependency_digest = self.route_dependency_digest();
        let routes = Self::renderable_routes(self.project, programs);
        self.add_routes(routes, previous, site, &dependency_digest)
    }

    pub(super) fn add_static_files(&mut self, has_tac_routes: bool) -> Result<(), Failure> {
        self.files
            .extend(SharedAssets::collect(self.snapshot_root)?);
        self.add_web_manifest_file()?;
        self.add_file(
            ".tachyon/navigation.css",
            WebCompiler::NAVIGATION_STYLESHEET.as_bytes().to_vec(),
        );
        self.add_file(
            "spa-renderer.js",
            WebCompiler::COMPATIBILITY_SPA_RUNTIME.as_bytes().to_vec(),
        );
        self.add_component_styles_file();
        self.add_client_runtime_file(has_tac_routes);
        Ok(())
    }

    pub(super) fn add_element_files(&mut self, names: &[String]) -> Result<(), Failure> {
        let renderer = ClientViewRenderer::new(&self.components, &self.services, &self.shared_data);
        let has_styles = !self.component_styles.is_empty();
        let elements = ExportedElements::new(renderer, &self.components, has_styles);
        self.files.extend(elements.files(names)?);
        Ok(())
    }

    pub(super) async fn add_service_files(&mut self) -> Result<(), Failure> {
        let jobs = BuildJobs::from_environment();
        let (dart, web, scripts) = futures_util::future::join3(
            jobs.run(self.dart_service_files()),
            self.web_assembly_service_files(&jobs),
            self.service_script_files(),
        )
        .await;
        let built = [dart?, web?, scripts?];
        self.add_built_services(built);
        Ok(())
    }

    pub(super) fn add_manifest_and_worker_files(
        &mut self,
        manifest: &impl Serialize,
        cache_rules: &str,
    ) -> Result<(), Failure> {
        self.add_file(
            "route-manifest.json",
            Self::pretty_json(manifest, "Route Manifest v1")?,
        );
        let api_reference = crate::handler::api_reference_files(self.project.route_graph())?;
        self.files.extend(api_reference);
        self.add_file(
            ".tachyon/register-sw.js",
            WebCompiler::SERVICE_WORKER_REGISTRATION.as_bytes().to_vec(),
        );
        let version = ServiceWorker::version(&self.files, self.build_config_digest.as_deref());
        let public_assets = ServiceWorker::assets(&self.files)?;
        let service_worker = WebCompiler::SERVICE_WORKER
            .replace("__VERSION__", &version)
            .replace("__CACHE_RULES__", cache_rules)
            .replace("__PUBLIC_ASSETS__", &public_assets);
        self.add_file("tachyon-sw.js", service_worker.into_bytes());
        let state = Self::pretty_json(&self.next_state, "incremental build state")?;
        self.add_file(".tachyon/build-state.json", state);
        Ok(())
    }

    pub(super) fn add_site_map_files(
        &mut self,
        site: &crate::native::SiteSettings,
    ) -> Result<(), Failure> {
        let routes = self.project.route_graph().routes();
        let site_map = crate::native::SiteMap::files(site, routes, &self.files)?;
        self.files.extend(site_map);
        Ok(())
    }

    pub(super) fn write_output(mut self) -> Result<BuildResult, Failure> {
        let mut files = std::mem::take(&mut self.files);
        files.sort_by_key(|value| PortablePath::of(&value.0));
        let compressed = CompressedFiles::create(&files, &self.output_directory, self.compress)?;
        self.finish_compression_phase();
        super::route_budget::BudgetCheck::verify(
            self.snapshot_root,
            &files,
            self.compress,
            &|path| compressed.sent_bytes(path),
        )?;
        super::ApiBaselineCheck::verify(self.snapshot_root, &files)?;
        let output_directory = std::mem::take(&mut self.output_directory);
        let stage = BuildStage::create(&output_directory)?;
        BuildStage::output_io(BuildStage::write(stage.path(), &files), &output_directory)?;
        let digest = BuildStage::output_io(BuildStage::digest(stage.path()), &output_directory)?;
        BuildStage::output_io(compressed.write(stage.path()), &output_directory)?;
        BuildStage::output_io(
            BuildStage::publish(stage, &output_directory),
            &output_directory,
        )?;
        self.finish_phase("output");
        Ok(BuildResult {
            output_directory,
            route_count: self.project.route_graph().routes().len(),
            sha256: digest,
            compiled_routes: self.compiled_routes,
            reused_routes: self.reused_routes,
            timings: self.timings,
            service_timings: self.service_timings,
            service_notes: self.service_notes,
            compression: compressed.totals(&files),
        })
    }

    fn finish_compression_phase(&mut self) {
        if self.compress {
            self.finish_phase("compression");
        }
    }

    fn compile_each_route(
        &self,
        programs: &mut BTreeMap<String, TemplateProgram>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), Failure> {
        let component_names = self.components.names();
        for route in self.project.route_graph().routes() {
            Self::compile_route(route, &component_names, programs, diagnostics)?;
        }
        Ok(())
    }

    fn compile_route(
        route: &RouteNode,
        component_names: &BTreeSet<String>,
        programs: &mut BTreeMap<String, TemplateProgram>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), Failure> {
        Self::route_view(route).map_or(Ok(()), |(bytes, source)| {
            Self::compile_view(
                route.route(),
                bytes,
                source,
                component_names,
                programs,
                diagnostics,
            )
        })
    }

    fn route_view(route: &RouteNode) -> Option<(&[u8], &str)> {
        Some((route.view_bytes()?, route.source_path()?))
    }

    fn compile_view(
        route: &str,
        bytes: &[u8],
        source: &str,
        component_names: &BTreeSet<String>,
        programs: &mut BTreeMap<String, TemplateProgram>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), Failure> {
        let template_source = PageState::read_template_source(bytes, source)?;
        let (template_source, inline_state) =
            Self::split_page_state(&template_source, source, diagnostics);
        let program =
            Self::compile_template(&template_source, source, component_names, diagnostics);
        let program = program.and_then(|program| {
            Self::composed_program(program, &inline_state, source, diagnostics)
        });
        programs.extend(program.map(|program| (String::from(route), program)));
        Ok(())
    }

    fn split_page_state(
        template_source: &str,
        source: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> (String, String) {
        match PageState::strip_scripts(template_source, source) {
            Ok(value) => value,
            Err(failure) => {
                diagnostics.extend_from_slice(failure.diagnostics());
                (
                    PageState::mask_script_blocks(template_source),
                    String::new(),
                )
            }
        }
    }

    fn compile_template(
        template_source: &str,
        source: &str,
        component_names: &BTreeSet<String>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<TemplateProgram> {
        match TemplateFrontend::compile(template_source, source, component_names) {
            Ok(program) => Some(program),
            Err(failure) => {
                diagnostics.extend_from_slice(failure.diagnostics());
                None
            }
        }
    }

    fn composed_program(
        program: TemplateProgram,
        inline_state: &str,
        source: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<TemplateProgram> {
        let composition = PageComposition::diagnostics(&program, inline_state, source);
        if !composition.is_empty() {
            diagnostics.extend(composition);
            return None;
        }
        Some(program)
    }

    fn programs_or_failure(
        programs: BTreeMap<String, TemplateProgram>,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<BTreeMap<String, TemplateProgram>, Failure> {
        if !diagnostics.is_empty() {
            return Err(Failure::new(diagnostics));
        }
        Ok(programs)
    }

    fn renderable_routes<'p>(
        project: &'p Project,
        programs: &'p BTreeMap<String, TemplateProgram>,
    ) -> Vec<(&'p RouteNode, &'p TemplateProgram, PathBuf)> {
        project
            .route_graph()
            .routes()
            .iter()
            .filter_map(|route| Self::renderable(route, programs))
            .collect()
    }

    fn renderable<'p>(
        route: &'p RouteNode,
        programs: &'p BTreeMap<String, TemplateProgram>,
    ) -> Option<(&'p RouteNode, &'p TemplateProgram, PathBuf)> {
        let program = programs.get(route.route())?;
        let output = route.template_output_path()?;
        Some((route, program, output))
    }

    fn add_routes(
        &mut self,
        routes: Vec<(&RouteNode, &TemplateProgram, PathBuf)>,
        previous: Option<&BuildState>,
        site: &crate::native::SiteSettings,
        dependency_digest: &str,
    ) -> Result<(), Failure> {
        for (route, program, output) in routes {
            self.add_route(route, program, output, previous, site, dependency_digest)?;
        }
        Ok(())
    }

    fn add_route(
        &mut self,
        route: &RouteNode,
        program: &TemplateProgram,
        output: PathBuf,
        previous: Option<&BuildState>,
        site: &crate::native::SiteSettings,
        dependency_digest: &str,
    ) -> Result<(), Failure> {
        let key = PortablePath::route_key(route.route());
        let input_sha = ContentDigest::route_input_sha(
            route.view_bytes().unwrap_or_else(|| unreachable!()),
            self.components.digest(),
            dependency_digest,
        );
        let prior = self.reusable_prior(previous, &key, route, &input_sha);
        let route_state = match prior {
            Some(prior) => self.reuse_route(prior)?,
            None => self.render_route(route, program, output, site, input_sha)?,
        };
        self.next_state.routes.insert(key, route_state);
        Ok(())
    }

    fn reusable_prior<'s>(
        &self,
        previous: Option<&'s BuildState>,
        key: &str,
        route: &RouteNode,
        input_sha: &str,
    ) -> Option<&'s RouteBuildState> {
        previous
            .and_then(|state| state.routes.get(key))
            .filter(|state| self.can_reuse(route, state, input_sha))
    }

    fn reuse_route(&mut self, prior: &RouteBuildState) -> Result<RouteBuildState, Failure> {
        self.add_reused_route_files(prior)?;
        Ok(prior.clone())
    }

    fn render_route(
        &mut self,
        route: &RouteNode,
        program: &TemplateProgram,
        output: PathBuf,
        site: &crate::native::SiteSettings,
        input_sha: String,
    ) -> Result<RouteBuildState, Failure> {
        let (artifacts, islands) = self.add_rendered_route_files(route, program, output, site)?;
        Ok(RouteBuildState {
            input_sha,
            artifacts,
            islands,
        })
    }

    fn route_dependency_digest(&self) -> String {
        let mut digest = Sha256::new();
        digest.update(self.tool_build.as_bytes());
        digest.update([0]);
        if let Some(config_digest) = &self.build_config_digest {
            digest.update(config_digest.as_bytes());
            digest.update([0]);
        }
        digest.update(self.services.digest().as_bytes());
        digest.update([0]);
        ContentDigest::hex(digest.finalize())
    }

    fn can_reuse(&self, route: &RouteNode, state: &RouteBuildState, input_sha: &str) -> bool {
        route.handlers().is_empty()
            && state.input_sha == input_sha
            && state.artifacts_are_intact(&self.output_directory)
    }

    fn add_reused_route_files(&mut self, prior: &RouteBuildState) -> Result<(), Failure> {
        for artifact in prior.artifacts.keys() {
            let bytes = BuildStage::output_io(
                fs::read(self.output_directory.join(artifact)),
                &self.output_directory,
            )?;
            self.files.push((PathBuf::from(artifact), bytes));
        }
        self.reused_routes += 1;
        Ok(())
    }

    fn add_rendered_route_files(
        &mut self,
        route: &RouteNode,
        program: &TemplateProgram,
        output: PathBuf,
        site: &crate::native::SiteSettings,
    ) -> Result<(BTreeMap<String, String>, BTreeSet<String>), Failure> {
        let output_portable = PortablePath::of(&output);
        let rendered = ClientViewRenderer::new(&self.components, &self.services, &self.shared_data)
            .render(
                program,
                &output_portable,
                &client_route_context(),
                route.route(),
            )?;
        Self::check_page_bindings(route.route(), rendered.page_bindings)?;
        let html = PageHead::apply(site, rendered.html, route.route());
        let html = self.add_runtime_links(&html);
        let key = PortablePath::route_key(route.route());
        let ir_path = PathBuf::from(format!(".tachyon/view-ir/{key}.json"));
        let map_path = PathBuf::from(format!(".tachyon/source-maps/{key}.map.json"));
        let html_bytes = html.into_bytes();
        let ir_bytes = Self::pretty_json(&program.view_ir(), "View IR v1")?;
        let map_bytes = Self::pretty_json(&rendered.source_map, "View Source Map v1")?;
        let artifacts = BTreeMap::from([
            (output_portable, ContentDigest::sha256_bytes(&html_bytes)),
            (
                PortablePath::of(&ir_path),
                ContentDigest::sha256_bytes(&ir_bytes),
            ),
            (
                PortablePath::of(&map_path),
                ContentDigest::sha256_bytes(&map_bytes),
            ),
        ]);
        self.files.push((output, html_bytes));
        self.files.push((ir_path, ir_bytes));
        self.files.push((map_path, map_bytes));
        self.compiled_routes += 1;
        Ok((artifacts, rendered.components))
    }

    fn check_page_bindings(route: &str, page_bindings: bool) -> Result<(), Failure> {
        if page_bindings {
            return Err(Failure::one(diagnostic(
                1306,
                format!("Route '{route}' binds on:<event> but has no client module."),
                Some(String::from(
                    "Add a colocated tac.js or tac.ts exporting the bound handlers.",
                )),
                None,
            )));
        }
        Ok(())
    }

    fn add_runtime_links(&self, html: &str) -> String {
        let mut html = PageHead::inject_before(html, "</head>", WebCompiler::NAVIGATION_LINK);
        if !self.component_styles.is_empty() {
            html = PageHead::inject_before(&html, "</head>", WebCompiler::COMPONENT_STYLE_LINK);
        }
        html = PageHead::inject_before(&html, "</head>", WebCompiler::SPECULATION_RULES);
        PageHead::inject_before(&html, "</body>", WebCompiler::SERVICE_WORKER_LINK)
    }

    fn add_file(&mut self, path: &str, bytes: Vec<u8>) {
        self.files.push((PathBuf::from(path), bytes));
    }

    fn add_component_styles_file(&mut self) {
        if !self.component_styles.is_empty() {
            let styles = self.component_styles.as_bytes().to_vec();
            self.add_file(".tachyon/components.css", styles);
        }
    }

    fn add_client_runtime_file(&mut self, has_tac_routes: bool) {
        if has_tac_routes {
            self.add_file(
                ".tachyon/tac-client.js",
                ClientViewRenderer::RUNTIME.as_bytes().to_vec(),
            );
        }
    }

    fn add_web_manifest_file(&mut self) -> Result<(), Failure> {
        let manifest_source = self
            .snapshot_root
            .join(crate::native::WebAppManifest::SOURCE_PATH);
        if !manifest_source.is_file() {
            return Ok(());
        }
        let bytes = fs::read(&manifest_source).map_err(|error| {
            Failure::one(diagnostic(
                1201,
                format!(
                    "Cannot read {}: {error}",
                    crate::native::WebAppManifest::SOURCE_PATH
                ),
                Some(String::from("Keep the manifest a readable regular file.")),
                None,
            ))
        })?;
        self.add_file(crate::native::WebAppManifest::OUTPUT_NAME, bytes);
        Ok(())
    }

    fn add_built_services(&mut self, built: [BuiltServices; 3]) {
        for services in built {
            self.files.extend(
                services.files.into_iter().map(|(name, bytes)| {
                    (PathBuf::from(format!(".tachyon/services/{name}")), bytes)
                }),
            );
            self.service_timings.extend(services.timings);
            self.service_notes.extend(services.notes);
        }
    }

    async fn dart_service_files(&self) -> Result<BuiltServices, Failure> {
        let started = Instant::now();
        let dart_services: Vec<(&str, &Path)> = self
            .services
            .iter()
            .filter_map(|(_, service)| Self::dart_source(service))
            .collect();
        self.build_dart_services(&dart_services, started).await
    }

    fn dart_source(service: &ServiceDefinition) -> Option<(&str, &Path)> {
        let source = service.script_path()?;
        ServiceScript::has_extension(source, "dart").then(|| (service.class(), source))
    }

    async fn build_dart_services(
        &self,
        dart_services: &[(&str, &Path)],
        started: Instant,
    ) -> Result<BuiltServices, Failure> {
        if dart_services.is_empty() {
            return Ok(BuiltServices::default());
        }
        let instances = self.services.names();
        let files =
            crate::tac_dart::build_all(self.snapshot_root, dart_services, &instances, self.profile)
                .await?;
        let timings = vec![(String::from("Dart"), started.elapsed())];
        Ok(BuiltServices {
            files,
            timings,
            notes: crate::tac_dart::compiler_note(self.profile).into_iter().collect(),
        })
    }

    async fn web_assembly_service_files(&self, jobs: &BuildJobs) -> Result<BuiltServices, Failure> {
        let web_services: Vec<(String, String, crate::tac_web::WebLanguage, PathBuf)> = self
            .services
            .iter()
            .filter_map(|(instance, service)| Self::web_service(instance, service))
            .collect();
        self.build_web_services(&web_services, jobs).await
    }

    fn web_service(
        instance: &str,
        service: &ServiceDefinition,
    ) -> Option<(String, String, crate::tac_web::WebLanguage, PathBuf)> {
        let (language, source) = service.web_assembly()?;
        let class = service.class().to_owned();
        Some((instance.to_owned(), class, language, source.to_path_buf()))
    }

    async fn build_web_services(
        &self,
        web_services: &[(String, String, crate::tac_web::WebLanguage, PathBuf)],
        jobs: &BuildJobs,
    ) -> Result<BuiltServices, Failure> {
        if web_services.is_empty() {
            return Ok(BuiltServices::default());
        }
        crate::tac_web::build_all(self.snapshot_root, web_services, jobs, self.profile).await
    }

    async fn service_script_files(&self) -> Result<BuiltServices, Failure> {
        let started = Instant::now();
        let mut built = BuiltServices::default();
        for (source, class) in self.script_services() {
            let bytes =
                ServiceScript::module(self.project.root(), self.snapshot_root, source).await?;
            built.files.push((format!("{class}.js"), bytes));
        }
        Self::add_script_timing(&mut built, started);
        Ok(built)
    }

    fn script_services(&self) -> Vec<(&Path, &str)> {
        self.services
            .iter()
            .filter_map(|(_, service)| Self::script_source(service))
            .collect()
    }

    fn script_source(service: &ServiceDefinition) -> Option<(&Path, &str)> {
        let source = service.script_path()?;
        let is_dart = ServiceScript::has_extension(source, "dart");
        (!is_dart).then(|| (source, service.class()))
    }

    fn add_script_timing(built: &mut BuiltServices, started: Instant) {
        if !built.files.is_empty() {
            built
                .timings
                .push((String::from("scripts"), started.elapsed()));
        }
    }

    fn pretty_json(value: &impl Serialize, name: &str) -> Result<Vec<u8>, Failure> {
        let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| {
            Failure::one(diagnostic(
                1201,
                format!("Cannot serialize {name}: {error}"),
                None,
                None,
            ))
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::SiteBuild;
    use crate::compiler::BuildOptions;
    use crate::compiler::build_state::BuildState;
    use crate::compiler::content_digest::ContentDigest;
    use crate::compiler::portable_path::PortablePath;
    use crate::compiler::tests::{build_default, write_styled_home_and_about};

    #[test]
    fn a_changed_ty_build_stops_route_reuse() {
        let root = tempfile::tempdir().expect("project");
        write_styled_home_and_about(root.path());
        let first = build_default(root.path()).expect("first build");
        let previous = BuildState::load(first.output_directory()).expect("build state");
        let project = crate::ProjectDiscovery::discover(root.path()).expect("project");
        let mut build =
            SiteBuild::discover(&project, &BuildOptions::default(), None).expect("site build");
        let home = project
            .route_graph()
            .routes()
            .iter()
            .find(|route| route.route() == "/")
            .expect("home route");
        let prior = previous
            .routes
            .get(&PortablePath::route_key("/"))
            .expect("home state");
        let reusable = |build: &SiteBuild| {
            let input_sha = ContentDigest::route_input_sha(
                home.view_bytes().expect("home view"),
                build.components.digest(),
                &build.route_dependency_digest(),
            );
            build.can_reuse(home, prior, &input_sha)
        };
        assert!(reusable(&build), "the same ty must reuse the route");
        build.tool_build.push_str("+rebuilt");
        assert!(!reusable(&build), "a new ty must rebuild the route");
    }
}
