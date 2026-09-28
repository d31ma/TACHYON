mod access_check;
mod capture_problem;
mod captured_file;
mod captured_sources;
mod companion_kind;
mod discovered;
mod entry_metadata;
mod handler_node;
mod native_companion;
mod project_discovery;
mod role_source;
mod route_graph;
mod route_node;
mod scheduled_worker;
mod snapshot_context;
mod source_root_step;
mod source_walk;
mod view_kind;
mod view_route;
mod view_source;
mod worker_schedules;

pub use companion_kind::CompanionKind;
pub use handler_node::HandlerNode;
pub use native_companion::NativeCompanion;
pub use project_discovery::ProjectDiscovery;
pub use route_graph::RouteGraph;
pub use route_node::RouteNode;
pub(crate) use scheduled_worker::ScheduledWorker;
pub use view_kind::ViewKind;

use crate::handler::{HandlerSource, OwnedSourceRoot};
use cap_std::fs::Dir;
use capture_problem::CaptureProblem;
use captured_file::CapturedFile;
use captured_sources::CapturedSources;
use discovered::Discovered;
use entry_metadata::EntryMetadata;
use role_source::RoleSource;
use snapshot_context::SnapshotContext;
use source_root_step::SourceRootStep;
use source_walk::SourceWalk;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use view_route::ViewRoute;
use view_source::ViewSource;
use worker_schedules::WorkerSchedules;

#[derive(Clone, Debug)]
pub struct Project {
    root: PathBuf,
    route_graph: RouteGraph,
    snapshot: Arc<OwnedSourceRoot>,
    capability: Arc<Dir>,
    middleware: Option<HandlerSource>,
    identity: Option<HandlerSource>,
    workers: Vec<ScheduledWorker>,
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        (&self.root, &self.route_graph) == (&other.root, &other.route_graph)
    }
}

impl Eq for Project {}

impl Project {
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub const fn route_graph(&self) -> &RouteGraph {
        &self.route_graph
    }

    pub(crate) fn snapshot_root(&self) -> &Path {
        self.snapshot.path()
    }

    pub(crate) fn capability(&self) -> Arc<Dir> {
        Arc::clone(&self.capability)
    }

    pub(crate) fn middleware(&self) -> Option<&HandlerSource> {
        self.middleware.as_ref()
    }

    pub(crate) fn identity_provider(&self) -> Option<&HandlerSource> {
        self.identity.as_ref()
    }

    pub(crate) fn workers(&self) -> &[ScheduledWorker] {
        &self.workers
    }

    pub(crate) fn invocation_sources(&self) -> impl Iterator<Item = &HandlerSource> + '_ {
        self.route_graph
            .routes()
            .iter()
            .flat_map(|route| route.handlers().iter().map(HandlerNode::source))
            .chain(self.middleware.iter())
            .chain(self.identity.iter())
            .chain(self.workers.iter().map(ScheduledWorker::source))
    }
}

pub(crate) fn companion_kind(name: &str) -> Option<CompanionKind> {
    CompanionKind::from_file_name(name)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{ProjectDiscovery, ViewKind};
    use crate::handler::HandlerSource;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    struct Sources;

    impl Sources {
        const PYTHON_NO_CONTENT: &str = "class Handler:\n    @staticmethod\n    def POST(request):\n        return {'status': 204, 'headers': {}, 'body': ''}\n";
    }

    fn write_file(root: &Path, relative: &str, contents: impl AsRef<[u8]>) {
        let path = root.join(relative);
        let parent = path.parent().expect("parent");
        fs::create_dir_all(parent).expect("directory");
        fs::write(&path, contents).expect("source");
    }

    fn project_with_home_page(view: &str) -> TempDir {
        let root = tempfile::tempdir().expect("project");
        write_file(root.path(), "client/pages/tac.html", view);
        root
    }

    fn discovery_error(root: &Path) -> String {
        ProjectDiscovery::discover(root)
            .expect_err("discovery should fail")
            .to_string()
    }

    #[test]
    fn routes_are_sorted_and_manifested() {
        let root = tempfile::tempdir().expect("project");
        write_file(root.path(), "client/pages/zeta/tac.html", "<p>Z</p>");
        write_file(root.path(), "client/pages/alpha/tac.html", "<p>A</p>");

        let project = ProjectDiscovery::discover(root.path()).expect("discovery");
        let routes = project.route_graph().routes();
        assert_eq!(routes[0].route(), "/alpha");
        assert_eq!(routes[1].route(), "/zeta");
        assert_eq!(
            routes[0].output_path(),
            Some(std::path::PathBuf::from("alpha/index.html"))
        );
        assert_eq!(project.route_graph().manifest().contract_version, 1);
        assert_eq!(project.root(), fs::canonicalize(root.path()).expect("root"));
        assert_eq!(routes[0].source_path(), Some("client/pages/alpha/tac.html"));
        assert_eq!(routes[0].view_kind(), Some(ViewKind::Tac));
        assert!(
            routes[0]
                .absolute_source_path()
                .is_some_and(std::path::Path::is_absolute)
        );
    }

    #[test]
    fn middleware_and_worker_schedules_share_one_ordered_owned_snapshot() {
        let root = project_with_home_page("<main>Page</main>");
        write_file(
            root.path(),
            "middleware.py",
            "class Middleware:\n    @staticmethod\n    def before(request):\n        return {'status': 204, 'headers': {}, 'body': ''}\n",
        );
        write_file(
            root.path(),
            "server/workers/zeta.py",
            Sources::PYTHON_NO_CONTENT,
        );
        write_file(
            root.path(),
            "server/workers/alpha.py",
            Sources::PYTHON_NO_CONTENT,
        );
        write_file(
            root.path(),
            ".tachyonrc",
            r#"{"workers":{"server/workers/zeta.py":{"every_seconds":20},"server/workers/alpha.py":{"every_seconds":10}}}"#,
        );

        let project = ProjectDiscovery::discover(root.path()).expect("discovery");
        let schedules = project
            .workers()
            .iter()
            .map(|worker| (worker.relative(), worker.every_seconds()))
            .collect::<Vec<_>>();
        assert_eq!(
            schedules,
            [
                ("server/workers/alpha.py", 10),
                ("server/workers/zeta.py", 20),
            ]
        );
        let execution_root = project.middleware().expect("middleware").execution_root();
        assert!(
            project
                .workers()
                .iter()
                .all(|worker| worker.source().execution_root() == execution_root)
        );
        let sources = project
            .invocation_sources()
            .map(HandlerSource::relative_path)
            .collect::<Vec<_>>();
        assert_eq!(
            sources,
            [
                "middleware.py",
                "server/workers/alpha.py",
                "server/workers/zeta.py",
            ]
        );
        assert_eq!(execution_root, project.snapshot_root());
    }

    #[test]
    fn every_shared_client_role_folder_is_captured() {
        let root = project_with_home_page("<money-panel hydrate=\"load\" />");
        write_file(
            root.path(),
            "client/components/money/panel/tac.html",
            "<main>{money.total}</main>",
        );
        for (folder, role, name) in [
            ("services", "Service", "MoneyService"),
            ("stores", "Store", "MoneyStore"),
            ("clients", "HTTPClient", "MoneyClient"),
            ("helpers", "Helper", "MoneyHelper"),
            ("converters", "Converter", "MoneySummaryConverter"),
            ("setups", "Setup", "MoneySetup"),
            ("entities", "Entity", "MoneyEntity"),
        ] {
            write_file(
                root.path(),
                &format!("client/{folder}/{name}.js"),
                format!("@{role}()\nclass {name} {{}}\n"),
            );
        }

        let project = ProjectDiscovery::discover(root.path()).expect("discovery");
        let staged = project.snapshot_root();
        for relative in [
            "client/services/MoneyService.js",
            "client/stores/MoneyStore.js",
            "client/clients/MoneyClient.js",
            "client/helpers/MoneyHelper.js",
            "client/converters/MoneySummaryConverter.js",
            "client/setups/MoneySetup.js",
            "client/entities/MoneyEntity.js",
        ] {
            assert!(
                staged.join(relative).exists(),
                "{relative} was not captured"
            );
        }
    }

    #[test]
    fn a_worker_schedules_itself_with_the_scheduled_marker() {
        let root = project_with_home_page("<main>Page</main>");
        write_file(
            root.path(),
            "server/workers/beat.py",
            format!(
                "@schedule({{\"fixedDelaySeconds\":45,\"initialDelaySeconds\":0}})\n{}",
                Sources::PYTHON_NO_CONTENT
            ),
        );

        let project = ProjectDiscovery::discover(root.path()).expect("discovery");
        let worker = project.workers().first().expect("scheduled worker");
        assert_eq!(worker.relative(), "server/workers/beat.py");
        assert_eq!(worker.every_seconds(), 45);
        assert_eq!(worker.initial_seconds(), 0);
    }

    #[test]
    fn a_worker_cannot_declare_its_cadence_twice() {
        let root = project_with_home_page("<main>Page</main>");
        write_file(
            root.path(),
            "server/workers/beat.py",
            format!(
                "@schedule({{\"fixedDelaySeconds\":45}})\n{}",
                Sources::PYTHON_NO_CONTENT
            ),
        );
        write_file(
            root.path(),
            ".tachyonrc",
            r#"{"workers":{"server/workers/beat.py":{"every_seconds":10}}}"#,
        );

        let rendered = discovery_error(root.path());
        assert!(rendered.contains("keep one"), "{rendered}");
    }

    #[test]
    fn a_scheduled_worker_must_exist_in_the_captured_server_snapshot() {
        let root = project_with_home_page("<main>Page</main>");
        write_file(
            root.path(),
            ".tachyonrc",
            r#"{"workers":{"server/workers/missing.py":{"every_seconds":10}}}"#,
        );

        let rendered = discovery_error(root.path());
        assert!(rendered.contains("TY2001"), "{rendered}");
        assert!(rendered.contains("server/workers/missing.py"), "{rendered}");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_root_middleware_is_rejected_without_following_it() {
        use std::os::unix::fs::symlink;

        let root = project_with_home_page("<main>Page</main>");
        let outside = tempfile::NamedTempFile::new().expect("outside");
        fs::write(outside.path(), "class Middleware: pass\n").expect("outside source");
        symlink(outside.path(), root.path().join("middleware.py")).expect("middleware link");

        let rendered = discovery_error(root.path());
        assert!(rendered.contains("TY1004"), "{rendered}");
        assert!(rendered.contains("middleware.py"), "{rendered}");
        assert!(!rendered.contains("Handler"), "link target was read");
    }

    #[test]
    fn a_source_root_blocked_by_a_file_is_found_on_every_platform() {
        let root = tempfile::tempdir().expect("workspace");
        fs::write(root.path().join("client"), "not a directory").expect("blocking file");
        let rendered = discovery_error(root.path());
        assert!(
            rendered.contains(
                "Source root 'client/pages' is blocked by 'client', which is not a directory."
            ),
            "{rendered}"
        );
    }

    #[test]
    fn invalid_static_segments_are_rejected() {
        for (segment, code) in [
            ("Not-Lower", "TY1005"),
            ("-start", "TY1005"),
            ("end-", "TY1005"),
            ("two--hyphens", "TY1005"),
            ("café", "TY1005"),
        ] {
            let root = tempfile::tempdir().expect("project");
            write_file(
                root.path(),
                &format!("client/pages/{segment}/tac.html"),
                "<p>Page</p>",
            );
            assert!(discovery_error(root.path()).contains(code));
        }
    }

    #[test]
    fn missing_invalid_and_empty_roots_have_stable_diagnostics() {
        let root = tempfile::tempdir().expect("workspace");
        let missing = root.path().join("missing");
        assert!(discovery_error(&missing).contains("TY1001"));

        let file = root.path().join("file");
        fs::write(&file, "not a directory").expect("file");
        assert!(discovery_error(&file).contains("TY1001"));
        assert!(discovery_error(root.path()).contains("TY1002"));

        fs::write(root.path().join("client"), "not a directory").expect("source root file");
        assert!(discovery_error(root.path()).contains("TY1001"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_sources_are_rejected() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().expect("workspace");
        let outside = root.path().join("outside.html");
        fs::write(&outside, "<main>Outside</main>").expect("outside source");
        fs::create_dir_all(root.path().join("client/pages")).expect("source root");
        symlink(&outside, root.path().join("client/pages/tac.html")).expect("symlink");
        assert!(discovery_error(root.path()).contains("TY1004"));
    }

    #[cfg(unix)]
    #[test]
    fn layer_discovery_rejects_external_and_cyclic_links_without_following_them() {
        use std::os::unix::fs::symlink;

        for layer in ["services", "stores", "clients", "helpers"] {
            let root = project_with_home_page("<main>Safe</main>");
            let outside = tempfile::tempdir().expect("outside");
            fs::write(outside.path().join("source.py"), "class XService: pass")
                .expect("outside source");
            let layer_root = root.path().join("server").join(layer);
            fs::create_dir_all(&layer_root).expect("layer root");
            symlink(outside.path(), layer_root.join("external")).expect("external link");
            symlink(&layer_root, layer_root.join("cycle")).expect("cyclic link");

            let rendered = discovery_error(root.path());
            assert!(rendered.contains("TY1004"), "{layer}: {rendered}");
            assert!(rendered.contains(&format!("server/{layer}/cycle")));
            assert!(rendered.contains(&format!("server/{layer}/external")));
            assert!(!rendered.contains("source.py"), "link target was traversed");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn opened_project_discovery_and_execution_ignore_planted_views_and_routes() {
        use crate::{HandlerCancellation, HandlerSupervisor, HandlerSupervisorOptions};
        use cap_std::ambient_authority;
        use cap_std::fs::Dir;
        use std::os::unix::fs::symlink;
        use tachyon_contracts::{HandlerRequest, HttpMethod};

        let workspace = tempfile::tempdir().expect("workspace");
        let root = workspace.path().join("project");
        for (relative, contents) in [
            ("client/pages/owned/tac.html", "<main>owned-view</main>"),
            (
                "server/controllers/OwnedController.py",
                "@request_route(\"/owned\")\n@public\nclass OwnedController:\n    @get_route\n    def get_owned(self):\n        return {'origin': 'owned-py'}\n",
            ),
            (
                "server/controllers/OwnedWriteController.js",
                "@RequestRoute(\"/owned\")\n@Public\nclass OwnedWriteController {\n  @PostRoute\n  postOwned() { return { origin: 'owned-js' } }\n}\n",
            ),
        ] {
            write_file(&root, relative, contents);
        }
        let canonical = fs::canonicalize(&root).expect("canonical root");
        let opened =
            Dir::open_ambient_dir(&canonical, ambient_authority()).expect("project handle");

        let retained = workspace.path().join("retained-project");
        fs::rename(&root, &retained).expect("move authored root after opening");
        let planted = tempfile::tempdir().expect("planted root");
        for (relative, contents) in [
            (
                "client/pages/canary/tac.html",
                "<main>external-canary-view</main>",
            ),
            (
                "server/controllers/CanaryController.py",
                "@request_route(\"/canary\")\n@public\nclass CanaryController:\n    @get_route\n    def get_canary(self):\n        return {}\n",
            ),
            (
                "server/controllers/OwnedController.py",
                "@request_route(\"/owned\")\n@public\nclass OwnedController:\n    @get_route\n    def get_owned(self):\n        return {'origin': 'planted-py'}\n",
            ),
            (
                "server/controllers/OwnedWriteController.js",
                "@RequestRoute(\"/owned\")\n@Public\nclass OwnedWriteController {\n  @PostRoute\n  postOwned() { return { origin: 'planted-js' } }\n}\n",
            ),
        ] {
            write_file(planted.path(), relative, contents);
        }
        symlink(planted.path(), &root).expect("replace ambient project root");

        let project = ProjectDiscovery::discover_opened(canonical, &opened).expect("snapshot");
        let routes = project.route_graph().routes();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].route(), "/owned");
        assert_eq!(
            routes[0].view_bytes(),
            Some(b"<main>owned-view</main>".as_slice())
        );
        assert_eq!(routes[0].handlers().len(), 2);
        let supervisor =
            HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor");
        for handler in routes[0].handlers() {
            let method = if handler.language() == crate::handler::HandlerLanguage::Python {
                HttpMethod::Get
            } else {
                HttpMethod::Post
            };
            let request = HandlerRequest::route(
                format!("project_snapshot_{}", handler.language().name()),
                "/owned",
                method,
            );
            let response = supervisor
                .invoke(handler.source(), &request, &HandlerCancellation::default())
                .await
                .expect("owned snapshot response");
            let body = response.body.expect("response body").data;
            assert!(body.contains("owned-"), "{body}");
            assert!(!body.contains("planted"), "{body}");
        }
        let rendered = format!("{project:?}");
        assert!(!rendered.contains("canary"), "planted project was observed");
    }

    #[test]
    fn application_modules_are_captured_without_framework_annotation_parsing() {
        let root = project_with_home_page("<main>Safe</main>");
        write_file(
            root.path(),
            "server/services/ZRepository.py",
            "class ZRepository: pass",
        );
        write_file(
            root.path(),
            "server/services/ARepository.py",
            "class ARepository: pass",
        );
        for layer in ["services", "stores", "clients", "helpers"] {
            write_file(
                root.path(),
                &format!("server/{layer}/invalid.py"),
                [0xff, 0xfe],
            );
        }

        let project =
            ProjectDiscovery::discover(root.path()).expect("ordinary application modules");
        assert_eq!(project.route_graph().routes().len(), 1);
    }

    #[test]
    fn a_page_directory_holds_only_its_view() {
        let root = project_with_home_page("<app-card />");
        for (relative, contents) in [
            ("client/pages/tac.js", "export default class {}"),
            ("client/pages/docs/tac.html", "<app-card />"),
            ("client/pages/docs/tac.css", "h1{}"),
            ("client/pages/docs/tac.swift", "final class Companion {}"),
            ("client/components/card/tac.html", "<p>card</p>"),
            ("client/components/card/tac.js", "export default class {}"),
            ("client/components/card/tac.css", "p{}"),
        ] {
            write_file(root.path(), relative, contents);
        }
        let failure =
            ProjectDiscovery::discover(root.path()).expect_err("page companions are rejected");
        let rendered = format!("{failure:?}");
        for path in [
            "client/pages/tac.js",
            "client/pages/docs/tac.css",
            "client/pages/docs/tac.swift",
        ] {
            assert!(rendered.contains(path), "{rendered}");
        }
        assert_eq!(
            rendered.matches("[49, 51, 48, 57]").count(),
            3,
            "{rendered}"
        );
        assert!(
            !rendered.contains("file: \"client/components"),
            "{rendered}"
        );
    }

    #[test]
    fn retired_route_files_are_refused_with_their_new_place() {
        for (relative, expected) in [
            ("server/routes/yon.html", "is inside server/routes"),
            ("server/routes/items/yon.js", "is inside server/routes"),
            ("server/controllers/yon.js", "is a yon route file"),
            (
                "server/controllers/OPTIONS.schema.json",
                "is an OPTIONS.schema.json file",
            ),
        ] {
            let root = project_with_home_page("<main>Tac</main>");
            write_file(root.path(), relative, "export class Handler {}");
            let error = discovery_error(root.path());
            assert!(error.contains("TY1310"), "{relative}: {error}");
            assert!(error.contains(expected), "{relative}: {error}");
        }
    }

    #[test]
    fn yon_html_is_refused_outside_the_retired_routes_folder() {
        for relative in [
            "server/yon.html",
            "server/controllers/yon.html",
            "server/shared/yon.html",
        ] {
            let root = project_with_home_page("<main>Tac</main>");
            write_file(root.path(), relative, "<main>Yon</main>");
            let error = discovery_error(root.path());
            assert!(error.contains("TY1008"), "{relative}: {error}");
            assert!(
                error.contains("Content-Type: text/html"),
                "{relative}: {error}"
            );
        }
    }

    #[test]
    fn controller_routes_and_pages_are_manifested_canonically() {
        let root = tempfile::tempdir().expect("project");
        for (relative, contents) in [
            (
                "server/controllers/ApiController.py",
                "@request_route(\"/api\")\n@public\nclass ApiController:\n    @get_route\n    def get_api(self):\n        return {}\n",
            ),
            ("client/pages/products/tac.html", "<main>Products</main>"),
            (
                "server/controllers/ProductsController.py",
                "@request_route(\"/products\")\n@public\nclass ProductsController:\n    @get_route\n    def get_products(self):\n        return []\n",
            ),
            (
                "server/controllers/ProductsWriteController.js",
                "@RequestRoute(\"/products\")\n@Public\nclass ProductsWriteController {\n  @PostRoute\n  postProduct() { return {} }\n}\n",
            ),
        ] {
            write_file(root.path(), relative, contents);
        }

        let project = ProjectDiscovery::discover(root.path()).expect("discovery");
        let routes = project.route_graph().routes();
        assert_eq!(routes.len(), 2);
        assert_eq!(routes[0].route(), "/api");
        assert!(routes[0].source_path().is_none());
        assert_eq!(routes[0].handlers().len(), 1);
        assert_eq!(routes[1].route(), "/products");
        assert_eq!(routes[1].handlers().len(), 2);
        assert_eq!(
            routes[1].handlers()[0].source_path(),
            "server/controllers/ProductsController.py"
        );
        assert_eq!(
            routes[1].handlers()[1].language(),
            crate::handler::HandlerLanguage::JavaScript
        );

        let manifest = project.route_graph().manifest();
        assert_eq!(manifest.routes[0].kind, tachyon_contracts::RouteKind::Api);
        assert_eq!(manifest.routes[0].methods.len(), 3);
        assert_eq!(manifest.routes[1].kind, tachyon_contracts::RouteKind::Page);
        assert_eq!(manifest.routes[1].methods.len(), 4);
        assert_eq!(manifest.routes[1].handlers[0].runtime, "python.v1");
    }

    #[test]
    fn dynamic_route_segments_are_discovered_with_their_parameters() {
        let root = tempfile::tempdir().expect("project");
        for (path, body) in [
            ("client/pages/tac.html", "<main>home</main>"),
            ("client/pages/items/_id/tac.html", "<main>item</main>"),
            (
                "client/pages/items/_id/reviews/_review/tac.html",
                "<main>r</main>",
            ),
        ] {
            write_file(root.path(), path, body);
        }
        let project = ProjectDiscovery::discover(root.path()).expect("discover");
        let routes = project.route_graph().routes();
        let find_route = |route: &str| {
            routes
                .iter()
                .find(|node| node.route() == route)
                .expect("route")
        };

        let item = find_route("/items/_id");
        assert!(item.is_dynamic());
        assert_eq!(item.parameters(), vec![String::from("id")]);
        assert!(item.output_path().is_none());

        let nested = find_route("/items/_id/reviews/_review");
        assert_eq!(
            nested.parameters(),
            vec![String::from("id"), String::from("review")]
        );

        let home = find_route("/");
        assert!(!home.is_dynamic());
        assert!(home.parameters().is_empty());
        assert!(home.output_path().is_some());
    }

    #[test]
    fn invalid_dynamic_segments_fail_closed() {
        for path in [
            "client/pages/_/tac.html",
            "client/pages/_1id/tac.html",
            "client/pages/_Id/tac.html",
            "client/pages/_id/_id/tac.html",
        ] {
            let root = tempfile::tempdir().expect("project");
            write_file(root.path(), path, "<main>x</main>");
            let error = ProjectDiscovery::discover(root.path())
                .err()
                .unwrap_or_else(|| panic!("{path} was accepted"));
            assert!(
                error.to_string().contains("TY1006") || error.to_string().contains("TY1005"),
                "{path}: {error}"
            );
        }
    }
}
