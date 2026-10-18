use crate::tac_cpp::CppFile;
use std::path::PathBuf;

pub(crate) struct CppYonHandler {
    pub(crate) path: PathBuf,
    pub(crate) file: CppFile,
    pub(crate) source: String,
}
