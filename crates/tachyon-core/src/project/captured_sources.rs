use super::{CaptureProblem, CaptureVendor, CapturedFile, ProjectDiscovery, SourceWalk};
use crate::Failure;
use crate::annotation_layout::{
    DesignCheck, DesignSwitches, DuplicationCheck, NativeModuleVendor, NativePackageFolder,
    RuleSwitch,
};
use crate::handler::OwnedSourceRoot;
use cap_std::fs::{Dir, DirEntry};
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(super) struct CapturedSources {
    pub(super) pages: Vec<CapturedFile>,
    components: Vec<CapturedFile>,
    shared: Vec<CapturedFile>,
    client_roles: Vec<CapturedFile>,
    pub(super) configs: Vec<CapturedFile>,
    pub(super) middleware: Option<CapturedFile>,
    pub(super) server: Vec<CapturedFile>,
    pub(super) controllers: Vec<CapturedFile>,
}

impl CapturedSources {
    const CLIENT_ROLE_ROOTS: [&str; 18] = [
        "client/features",
        "client/services",
        "client/controllers",
        "client/stores",
        "client/repositories",
        "client/remotes",
        "client/clients",
        "client/delegates",
        "client/helpers",
        "client/converters",
        "client/mappers",
        "client/tests",
        "client/setups",
        "client/configurations",
        "client/entities",
        "client/models",
        "client/errors",
        crate::annotation_layout::UtilityLayout::REMOVED_CLIENT_ROOT,
    ];
    const CONFIG_FILES: [&str; 3] = ["package.json", "client/manifest.json", ".tachyonrc"];
    const COMPONENT_ROOT: &str = "client/components";
    const SHARED_ROOT: &str = "client/shared";
    const SERVER_ROOT: &str = "server";
    const MIDDLEWARE_PREFIX: &str = "middleware.";

    pub(super) fn capture(project: &Dir, diagnostics: &mut Vec<Diagnostic>) -> Self {
        let pages =
            SourceWalk::capture(project, Path::new(ProjectDiscovery::TAC_ROOT), diagnostics);
        let components = SourceWalk::capture(project, Path::new(Self::COMPONENT_ROOT), diagnostics);
        let shared = SourceWalk::capture(project, Path::new(Self::SHARED_ROOT), diagnostics);
        let client_roles = Self::CLIENT_ROLE_ROOTS
            .iter()
            .flat_map(|root| SourceWalk::capture(project, Path::new(root), diagnostics))
            .collect();
        let configs = Self::CONFIG_FILES
            .iter()
            .filter_map(|relative| {
                CapturedFile::project_file(project, Path::new(relative), diagnostics)
            })
            .collect();
        let middleware = Self::root_middleware(project, diagnostics);
        let server = SourceWalk::capture(project, Path::new(Self::SERVER_ROOT), diagnostics);
        let controllers =
            Self::files_beneath(&server, Path::new(ProjectDiscovery::CONTROLLER_ROOT));
        Self {
            pages,
            components,
            shared,
            client_roles,
            configs,
            middleware,
            server,
            controllers,
        }
    }

    pub(super) fn stage(
        &self,
        source_root: &OwnedSourceRoot,
        cohesion: RuleSwitch,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let vendor = Self::vendor_context(&self.configs, &self.shared, diagnostics);
        let roles = self.client_roles.iter().chain(&self.server);
        let forwards = crate::delegate_forwards::DelegateForwards::new(
            roles.map(|file| (file.relative.as_path(), file.bytes.as_slice())),
        );
        self.pages
            .iter()
            .chain(&self.components)
            .chain(&self.shared)
            .chain(&self.client_roles)
            .chain(&self.configs)
            .chain(&self.middleware)
            .chain(&self.server)
            .for_each(|file| {
                let checked = Self::checked_file(&vendor, file, cohesion);
                let written = forwards.staged(&file.relative, &file.bytes);
                Self::stage_file(file, source_root, (checked, written), diagnostics);
            });
        diagnostics.extend(Self::native_module_problems(&self.server));
    }

    fn checked_file(
        vendor: &CaptureVendor,
        file: &CapturedFile,
        cohesion: RuleSwitch,
    ) -> Result<(), Failure> {
        if NativePackageFolder::contains(&file.relative) {
            return NativePackageFolder::check_file(&file.relative, &file.bytes);
        }
        vendor.check_file(file, cohesion)
    }

    fn native_module_problems(server: &[CapturedFile]) -> Vec<Diagnostic> {
        let paths = server.iter().map(|file| file.relative.as_path());
        let files = server
            .iter()
            .map(|file| (file.relative.as_path(), file.bytes.as_slice()));
        [
            NativePackageFolder::check_module(paths),
            NativeModuleVendor::check(files),
        ]
        .into_iter()
        .filter_map(Result::err)
        .flat_map(|failure| failure.diagnostics().to_vec())
        .collect()
    }

    fn authored_server_files(&self) -> impl Iterator<Item = &CapturedFile> + Clone {
        self.server
            .iter()
            .filter(|file| !NativeModuleVendor::contains(&file.relative))
    }

    fn convention_files(&self) -> impl Iterator<Item = &CapturedFile> + Clone {
        self.server
            .iter()
            .filter(|file| !NativePackageFolder::contains(&file.relative))
    }

    pub(super) fn check_duplication(&self, switch: RuleSwitch, diagnostics: &mut Vec<Diagnostic>) {
        let files = self
            .pages
            .iter()
            .chain(&self.components)
            .chain(&self.shared)
            .chain(&self.client_roles)
            .chain(&self.middleware)
            .chain(self.convention_files())
            .map(|file| (file.relative.as_path(), file.bytes.as_slice()));
        let found = DuplicationCheck::diagnostics(files.clone(), switch);
        diagnostics.extend(crate::fsharp_source::FsharpSpans::restored(&found, files));
    }

    pub(super) fn check_design(
        &self,
        switches: &DesignSwitches,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let files = self
            .client_roles
            .iter()
            .chain(self.convention_files())
            .chain(&self.pages)
            .chain(&self.components)
            .chain(&self.shared)
            .chain(&self.middleware)
            .map(|file| (file.relative.as_path(), file.bytes.as_slice()));
        let found = DesignCheck::diagnostics(files.clone(), switches);
        diagnostics.extend(crate::fsharp_source::FsharpSpans::restored(&found, files));
    }

    pub(super) fn check_secrets(&self, diagnostics: &mut Vec<Diagnostic>) {
        let files = self
            .pages
            .iter()
            .chain(&self.components)
            .chain(&self.shared)
            .chain(&self.client_roles)
            .chain(&self.configs)
            .chain(&self.middleware)
            .chain(self.authored_server_files())
            .map(|file| (file.relative.as_path(), file.bytes.as_slice()));
        diagnostics.extend(crate::safety::SecretCheck::diagnostics(files));
    }

    pub(super) fn check_accessibility(&self, project: &Dir, diagnostics: &mut Vec<Diagnostic>) {
        let views = self
            .pages
            .iter()
            .chain(&self.components)
            .map(|file| (file.relative.as_path(), file.bytes.as_slice()));
        let switch = crate::template::AccessibilityCheck::switch(project);
        diagnostics.extend(crate::template::AccessibilityCheck::diagnostics(
            views, switch,
        ));
    }

    pub(super) fn check_code_safety(&self, project: &Dir, diagnostics: &mut Vec<Diagnostic>) {
        let code = || {
            self.client_roles
                .iter()
                .chain(&self.components)
                .chain(&self.shared)
                .chain(&self.middleware)
                .chain(self.authored_server_files())
                .map(|file| (file.relative.as_path(), file.bytes.as_slice()))
        };
        diagnostics.extend(crate::safety::EvalCheck::diagnostics(code()));
        diagnostics.extend(crate::safety::QueryCheck::diagnostics(code()));
        diagnostics.extend(crate::safety::ServiceReachCheck::diagnostics(code()));
        let tac = crate::safety::DeclaredHosts::tac(project);
        let yon = crate::safety::DeclaredHosts::yon(project);
        diagnostics.extend(crate::safety::HostCheck::new(&tac, &yon).diagnostics(code()));
    }

    fn vendor_context(
        configs: &[CapturedFile],
        shared: &[CapturedFile],
        diagnostics: &mut Vec<Diagnostic>,
    ) -> CaptureVendor {
        CaptureVendor::from_files(configs, shared).unwrap_or_else(|failure| {
            diagnostics.extend_from_slice(failure.diagnostics());
            CaptureVendor::default()
        })
    }

    fn stage_file(
        file: &CapturedFile,
        source_root: &OwnedSourceRoot,
        (checked, written): (Result<(), Failure>, Result<Option<String>, Failure>),
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let bytes = written
            .as_ref()
            .ok()
            .and_then(Option::as_deref)
            .map_or(file.bytes.as_slice(), str::as_bytes);
        let staged = source_root.stage(&file.relative, bytes, &file.portable());
        let forwarded = written.map(|_| ());
        [checked, forwarded, staged]
            .into_iter()
            .filter_map(Result::err)
            .for_each(|failure: Failure| diagnostics.extend_from_slice(failure.diagnostics()));
    }

    fn root_middleware(project: &Dir, diagnostics: &mut Vec<Diagnostic>) -> Option<CapturedFile> {
        let entries = Self::sorted_root_entries(project)
            .map_err(|error| {
                diagnostics.push(CaptureProblem::error(format!(
                    "Cannot enumerate project root: {error}"
                )));
            })
            .ok()?;
        let name = entries
            .iter()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .find(|name| name.starts_with(Self::MIDDLEWARE_PREFIX))?;
        CapturedFile::project_file(project, Path::new(&name), diagnostics)
    }

    fn sorted_root_entries(project: &Dir) -> std::io::Result<Vec<DirEntry>> {
        let mut entries = project.entries()?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(DirEntry::file_name);
        Ok(entries)
    }

    fn files_beneath(files: &[CapturedFile], root: &Path) -> Vec<CapturedFile> {
        files
            .iter()
            .filter(|file| {
                file.relative
                    .strip_prefix(root)
                    .is_ok_and(|beneath| !beneath.as_os_str().is_empty())
            })
            .cloned()
            .collect()
    }
}
