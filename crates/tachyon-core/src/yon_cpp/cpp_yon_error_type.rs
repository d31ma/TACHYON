use super::CppYonErrorTypes;
use crate::Failure;
use crate::tac_cpp::{CppClass, CppFailure, CppFile, CppLowering, CppMarkerCheck};
use std::path::{Path, PathBuf};

pub(crate) struct CppYonErrorType {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) at: usize,
    pub(crate) bases: Vec<String>,
    pub(crate) lowered: String,
}

impl CppYonErrorType {
    const FOLDER: &'static str = "server/errors";
    const BASE_WORDS: [&'static str; 4] = ["public", "protected", "private", "virtual"];
    const PLACE: &'static str = "A C++ error type is one class directly in server/errors, named after its file, for example `class OrderNotFound : public std::runtime_error { public: using std::runtime_error::runtime_error; };`";
    const PLAIN: &'static str = "A C++ error type is a plain class without Tachyon markers. Put [[ErrorHandler]] on its handler in server/controllers";

    pub(crate) fn read(path: &Path, source: &str) -> Result<Self, Failure> {
        let file = CppFile::read(path, source)?;
        let class = &file.class;
        let named = path
            .file_stem()
            .is_some_and(|stem| stem == class.name.as_str());
        let placed = [path.parent() == Some(Path::new(Self::FOLDER)), named] == [true, true];
        let plain = class
            .every_marker()
            .iter()
            .all(|marker| CppMarkerCheck::is_standard(&marker.name));
        let bases = Self::bases(source, class);
        let problem = [
            (placed, Self::PLACE),
            (plain, Self::PLAIN),
            (!bases.is_empty(), CppYonErrorTypes::DERIVED),
        ]
        .into_iter()
        .find(|(passed, _)| !passed);
        problem.map_or(Ok(()), |(_, message)| {
            Err(CppFailure::unreadable(path, class.name_at, message))
        })?;
        Ok(Self {
            name: class.name.clone(),
            path: path.to_path_buf(),
            at: class.name_at,
            bases,
            lowered: CppLowering::model(&file, source),
        })
    }

    fn bases(source: &str, class: &CppClass) -> Vec<String> {
        let start = class.name_at + class.name.len();
        let between = source.get(start..class.body.start).unwrap_or_default();
        let listed = between.trim().trim_start_matches("final").trim();
        listed
            .strip_prefix(':')
            .unwrap_or_default()
            .split(',')
            .map(Self::base_name)
            .filter(|name| !name.is_empty())
            .collect()
    }

    fn base_name(written: &str) -> String {
        let words: Vec<&str> = written
            .split_whitespace()
            .filter(|word| !Self::BASE_WORDS.contains(word))
            .collect();
        words.join(" ").trim_start_matches("::").to_owned()
    }
}
