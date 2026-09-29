use super::java_member::JavaMember;
use std::path::PathBuf;

pub(super) struct JavaService {
    pub(super) instance: String,
    pub(super) class: String,
    pub(super) members: Vec<JavaMember>,
    pub(super) required: Vec<String>,
    pub(super) path: PathBuf,
    pub(super) source: String,
}
