use super::{
    CaptureProblem, CapturedSources, Discovered, Project, SnapshotContext, WorkerSchedules,
};
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::OwnedSourceRoot;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default)]
pub struct ProjectDiscovery;

impl ProjectDiscovery {
    pub(super) const TAC_ROOT: &str = "client/pages";
    pub(super) const CONTROLLER_ROOT: &str = "server/controllers";

    pub fn discover(root: impl AsRef<Path>) -> Result<Project, Failure> {
        let canonical_root = Self::canonical_root(root.as_ref())?;
        let project =
            Dir::open_ambient_dir(&canonical_root, ambient_authority()).map_err(|error| {
                Failure::one(CaptureProblem::error(format!(
                    "Cannot open project root '{}': {error}",
                    canonical_root.display()
                )))
            })?;
        Self::discover_opened(canonical_root, &project)
    }

    pub(super) fn discover_opened(
        canonical_root: PathBuf,
        project: &Dir,
    ) -> Result<Project, Failure> {
        crate::ApplicationConfig::load_directory_for_client(project)?;
        let mut found = Discovered::default();
        let mut captured = CapturedSources::capture(project, &mut found.diagnostics);
        let source_root = OwnedSourceRoot::new_project(Self::CONTROLLER_ROOT)?;
        let cohesion = crate::annotation_layout::RuleSwitch::cohesion(project);
        captured.stage(&source_root, cohesion, &mut found.diagnostics);
        let duplication = crate::annotation_layout::RuleSwitch::duplication(project);
        captured.check_duplication(duplication, &mut found.diagnostics);
        let design = crate::annotation_layout::DesignSwitches::read(project);
        captured.check_design(&design, &mut found.diagnostics);
        captured.check_secrets(&mut found.diagnostics);
        captured.check_code_safety(project, &mut found.diagnostics);
        captured.check_accessibility(project, &mut found.diagnostics);
        let context = SnapshotContext {
            project_root: &canonical_root,
            project,
            source_root: &source_root,
        };
        found.inspect(&context, &captured);
        let identity = found.identity_provider();
        let middleware = context.middleware(captured.middleware.take(), &mut found.diagnostics);
        let workers = WorkerSchedules::discover(&context, &captured, &mut found.diagnostics);
        let route_graph = found.into_route_graph()?;
        let capability = Self::retain_capability(project)?;
        Ok(Project {
            root: canonical_root,
            route_graph,
            snapshot: source_root,
            capability,
            middleware,
            identity,
            workers,
        })
    }

    fn canonical_root(root: &Path) -> Result<PathBuf, Failure> {
        let canonical = fs::canonicalize(root).map_err(|error| {
            Failure::one(diagnostic(
                1001,
                format!("Cannot open project root '{}': {error}", root.display()),
                Some(String::from("Pass an existing readable project directory.")),
                None,
            ))
        })?;
        let metadata = fs::metadata(&canonical).map_err(|error| {
            Failure::one(CaptureProblem::error(format!(
                "Cannot inspect project root '{}': {error}",
                canonical.display()
            )))
        })?;
        if !metadata.is_dir() {
            return Err(Failure::one(CaptureProblem::error(format!(
                "Project root '{}' is not a directory.",
                canonical.display()
            ))));
        }
        Ok(canonical)
    }

    fn retain_capability(project: &Dir) -> Result<Arc<Dir>, Failure> {
        let copy = project.try_clone().map_err(|error| {
            Failure::one(CaptureProblem::error(format!(
                "Cannot retain the project capability: {error}"
            )))
        })?;
        Ok(Arc::new(copy))
    }
}
