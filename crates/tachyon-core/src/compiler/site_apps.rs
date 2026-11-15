use crate::failure::diagnostic;
use crate::native::SiteSettings;
use crate::routing::MountPath;
use crate::{Failure, Project, ProjectDiscovery};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) struct SiteApps {
    apps: Vec<(MountPath, PathBuf)>,
}

impl SiteApps {
    const MAX_APPS: usize = 32;
    const CODE: u16 = 1601;
    pub(super) const LIST_PATH: &'static str = ".tachyon/apps.json";
    const HELP: &'static str = "tac.apps maps each page app's path prefix to its project folder, such as {\"/chat\": \"../chat\"}, in the manifest of the shell, the app at / (ADR 0139).";

    pub(super) fn resolve(shell: &Project, site: &SiteSettings) -> Result<Self, Failure> {
        Self::check_shell(shell, &site.apps)?;
        Self::check_count(&site.apps)?;
        let apps = Self::resolved(shell.root(), &site.apps)?;
        Self::check_nesting(&apps)?;
        Self::check_shell_routes(shell, &apps)?;
        Ok(Self { apps })
    }

    pub(super) const fn none() -> Self {
        Self { apps: Vec::new() }
    }

    pub(super) fn refuse_native(
        &self,
        target: Option<tachyon_contracts::NativeTarget>,
    ) -> Result<(), Failure> {
        Self::check_native([target.is_some(), !self.is_empty()] == [true, true])
    }

    fn check_native(refused: bool) -> Result<(), Failure> {
        if !refused {
            return Ok(());
        }
        Err(Self::failure(
            "A native app of a site with tac.apps is not built yet: native apps will hold the whole site in a later release. Build this target from a project without tac.apps for now.",
        ))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    pub(super) fn prefixes(&self) -> Vec<String> {
        self.apps
            .iter()
            .map(|(mount, _)| String::from(mount.prefix()))
            .collect()
    }

    pub(super) fn list_file(&self) -> Result<Vec<u8>, Failure> {
        let list = serde_json::json!({ "apps": self.prefixes() });
        serde_json::to_vec_pretty(&list).map_err(|error| Self::failure(&error.to_string()))
    }

    pub(super) fn projects(&self) -> Result<Vec<Project>, Failure> {
        self.apps
            .iter()
            .map(|(mount, root)| Self::project(mount, root))
            .collect()
    }

    fn project(mount: &MountPath, root: &Path) -> Result<Project, Failure> {
        let project = ProjectDiscovery::discover(root)?;
        Self::check_mount(mount, &project)?;
        Ok(project)
    }

    fn check_mount(declared: &MountPath, project: &Project) -> Result<(), Failure> {
        if project.mount() == declared {
            return Ok(());
        }
        Err(Self::failure(&format!(
            "tac.apps names {} for the project {}, but that project's manifest has tac.mount \"{}\". Both must name the same prefix.",
            declared.prefix(),
            project.root().display(),
            project.mount().prefix()
        )))
    }

    fn check_shell(shell: &Project, apps: &BTreeMap<String, String>) -> Result<(), Failure> {
        let allowed = [apps.is_empty(), shell.mount().is_root()].contains(&true);
        if allowed {
            return Ok(());
        }
        Err(Self::failure(
            "A page app with tac.mount cannot list tac.apps. Only the shell, the app at /, lists the site's page apps.",
        ))
    }

    fn check_count(apps: &BTreeMap<String, String>) -> Result<(), Failure> {
        if apps.len() <= Self::MAX_APPS {
            return Ok(());
        }
        Err(Self::failure("tac.apps lists more than 32 page apps."))
    }

    fn resolved(
        shell_root: &Path,
        apps: &BTreeMap<String, String>,
    ) -> Result<Vec<(MountPath, PathBuf)>, Failure> {
        apps.iter()
            .map(|(prefix, folder)| Self::resolved_app(shell_root, prefix, folder))
            .collect()
    }

    fn resolved_app(
        shell_root: &Path,
        prefix: &str,
        folder: &str,
    ) -> Result<(MountPath, PathBuf), Failure> {
        let mount = MountPath::parse(prefix).map_err(|message| Self::failure(&message))?;
        let root = Self::app_root(shell_root, folder)?;
        Ok((mount, root))
    }

    fn app_root(shell_root: &Path, folder: &str) -> Result<PathBuf, Failure> {
        let relative = Self::relative_folder(folder)?;
        let root = std::fs::canonicalize(shell_root.join(relative)).map_err(|error| {
            Self::failure(&format!(
                "tac.apps names the folder '{folder}', which cannot be opened: {error}"
            ))
        })?;
        Self::outside_shell(shell_root, folder, root)
    }

    fn relative_folder(folder: &str) -> Result<&Path, Failure> {
        let path = Path::new(folder);
        if path.is_relative() {
            return Ok(path);
        }
        Err(Self::failure(&format!(
            "tac.apps names the folder '{folder}'. Name it relative to the shell's project folder, such as ../chat."
        )))
    }

    fn outside_shell(shell_root: &Path, folder: &str, root: PathBuf) -> Result<PathBuf, Failure> {
        if !root.starts_with(shell_root) {
            return Ok(root);
        }
        Err(Self::failure(&format!(
            "tac.apps names the folder '{folder}', which is inside the shell's project. Keep each page app in its own project folder beside the shell."
        )))
    }

    fn check_nesting(apps: &[(MountPath, PathBuf)]) -> Result<(), Failure> {
        let nested = apps
            .iter()
            .flat_map(|(outer, _)| apps.iter().map(move |(inner, _)| (outer, inner)))
            .find(|(outer, inner)| Self::nests(outer, inner));
        nested.map_or(Ok(()), |(outer, inner)| {
            Err(Self::failure(&format!(
                "tac.apps lists {} inside {}. Page app prefixes cannot nest.",
                inner.prefix(),
                outer.prefix()
            )))
        })
    }

    fn nests(outer: &MountPath, inner: &MountPath) -> bool {
        [outer != inner, outer.contains_route(inner.prefix())] == [true, true]
    }

    fn check_shell_routes(shell: &Project, apps: &[(MountPath, PathBuf)]) -> Result<(), Failure> {
        let claimed = shell
            .route_graph()
            .routes()
            .iter()
            .map(crate::RouteNode::route)
            .find(|route| Self::claimed_by_app(route, apps));
        claimed.map_or(Ok(()), |route| {
            Err(Self::failure(&format!(
                "The shell has the page {route}, but a page app owns that prefix. Move the page into that app."
            )))
        })
    }

    fn claimed_by_app(route: &str, apps: &[(MountPath, PathBuf)]) -> bool {
        apps.iter().any(|(mount, _)| mount.contains_route(route))
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
