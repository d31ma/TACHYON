use super::java_service::JavaService;
use std::path::PathBuf;

pub(super) struct JavaPageSources {
    pub(super) services: Vec<JavaService>,
    pub(super) page: Option<(PathBuf, String)>,
}
