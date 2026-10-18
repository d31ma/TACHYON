use super::{CppYonAccess, CppYonPath, CppYonRoute, CppYonWorker};
use crate::Failure;
use crate::tac_cpp::{CppAttribute, CppClass, CppFailure, CppFile, CppMarkerCheck};
use crate::yon::plan::Route;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) struct CppYonController {
    pub(crate) file: CppFile,
    pub(crate) source: String,
    pub(crate) path: PathBuf,
    pub(crate) routes: Vec<Route>,
}

impl CppYonController {
    pub(crate) const CODE: u16 = 2015;
    const ROUTE_MARKER: &'static str = "RequestRoute";
    const MAX_ROUTES: usize = 64;
    const NOT_A_CONTROLLER: &'static str = "A C++ file in server/controllers is a controller: write `class [[RequestRoute(\"/orders\")]] OrderController`. A [[Service]] goes in server/services and a [[Helper]] in server/helpers, and a controller reaches them with [[Needs]] fields. Middleware, error handlers, sockets, relays and test units come in a later step (ADR 0099)";
    const ROUTE_COUNT: &'static str = "A [[RequestRoute]] controller declares from 1 to 64 route methods. Mark a method with [[GetRoute]], [[PostRoute]], [[PutRoute]], [[PatchRoute]] or [[DeleteRoute]]: [[GetRoute]] means the method answers GET requests";
    const TWICE: &'static str = "Two methods answer the same HTTP method and path. Each route path and HTTP method pair needs one method";

    pub(crate) fn read(path: &Path, source: &str) -> Result<Self, Failure> {
        Self::routed(CppYonWorker::is_worker(path), path, source)
    }

    fn routed(worker: bool, path: &Path, source: &str) -> Result<Self, Failure> {
        if worker {
            return CppYonWorker::read(path, source);
        }
        Self::controller(path, source)
    }

    fn controller(path: &Path, source: &str) -> Result<Self, Failure> {
        let file = CppFile::read(path, source)?;
        CppMarkerCheck::check(path, &file.class)?;
        let marker = Self::route_marker(path, &file.class)?;
        let base = CppYonPath::of(path, marker)?;
        let access = CppYonAccess::read(&file.class.markers);
        let routes = CppYonRoute::all(path, &file.class, &base, &access)?;
        Self::check_routes(path, &routes)?;
        Ok(Self {
            file,
            source: source.to_owned(),
            path: path.to_path_buf(),
            routes,
        })
    }

    fn route_marker<'a>(path: &Path, class: &'a CppClass) -> Result<&'a CppAttribute, Failure> {
        class
            .markers
            .iter()
            .find(|marker| marker.name == Self::ROUTE_MARKER)
            .ok_or_else(|| {
                CppFailure::coded(Self::CODE, path, class.name_at, Self::NOT_A_CONTROLLER)
            })
    }

    fn check_routes(path: &Path, routes: &[Route]) -> Result<(), Failure> {
        let counted = (1..=Self::MAX_ROUTES).contains(&routes.len());
        let pairs: BTreeSet<(&str, Option<&str>)> = routes
            .iter()
            .map(|route| (route.verb.as_str(), route.pattern.as_deref()))
            .collect();
        let unique = pairs.len() == routes.len();
        let problem = [(counted, Self::ROUTE_COUNT), (unique, Self::TWICE)]
            .into_iter()
            .find(|(passed, _)| !passed);
        problem.map_or(Ok(()), |(_, message)| {
            Err(CppFailure::coded(Self::CODE, path, 0, message))
        })
    }
}
