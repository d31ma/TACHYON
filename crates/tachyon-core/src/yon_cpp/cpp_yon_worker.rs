use super::CppYonController;
use crate::Failure;
use crate::schedule::Schedule;
use crate::tac_cpp::{CppClass, CppFailure, CppFile, CppMarkerCheck, CppMemberKind};
use crate::yon::plan::Route;
use std::path::Path;

pub(crate) struct CppYonWorker;

impl CppYonWorker {
    const FOLDER: &'static str = "server/workers";
    const CLASS: &'static str = "Handler";
    const METHOD: &'static str = "POST";
    const SCHEDULE: &'static str = "Schedule";
    const SHAPE: &'static str = "A C++ worker in server/workers is a `class Handler` with a `POST()` method that takes no parameters; the runtime calls it on the worker's schedule, for example `class [[Schedule({\"fixedDelaySeconds\":300})]] Handler { public: void POST() {} };`";

    pub(crate) fn is_worker(path: &Path) -> bool {
        path.starts_with(Self::FOLDER)
    }

    pub(crate) fn read(path: &Path, source: &str) -> Result<CppYonController, Failure> {
        let file = CppFile::read(path, source)?;
        CppMarkerCheck::check(path, &file.class)?;
        let route = Self::route(path, &file.class)?;
        Ok(CppYonController {
            file,
            source: source.to_owned(),
            path: path.to_path_buf(),
            routes: vec![route],
        })
    }

    pub(crate) fn schedule(relative: &str, source: &str) -> Result<Option<Schedule>, Failure> {
        let file = CppFile::read(Path::new(relative), source)?;
        let marker = file
            .class
            .markers
            .iter()
            .find(|marker| marker.name == Self::SCHEDULE);
        marker
            .map(|marker| Schedule::written(relative, marker.at, &marker.arguments))
            .transpose()
    }

    fn route(path: &Path, class: &CppClass) -> Result<Route, Failure> {
        let named = class.name == Self::CLASS;
        let post = class.members.iter().find(|member| {
            [
                named,
                member.kind == CppMemberKind::Method,
                member.name == Self::METHOD,
                member.parameters.is_empty(),
            ] == [true, true, true, true]
        });
        post.map(|member| Route {
            verb: String::from(Self::METHOD),
            method: member.name.clone(),
            returns: member.plain_type(),
            ..Route::default()
        })
        .ok_or_else(|| CppFailure::coded(CppYonController::CODE, path, class.name_at, Self::SHAPE))
    }
}
