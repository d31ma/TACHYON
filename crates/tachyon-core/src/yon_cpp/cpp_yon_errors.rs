use super::{CppYonController, CppYonErrorMethod, CppYonErrorTypes, CppYonHandler};
use crate::Failure;
use crate::tac_cpp::{CppFailure, CppFile, CppFolder, CppMarkerCheck};
use crate::yon::plan::ErrorResponse;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) struct CppYonErrors {
    types: CppYonErrorTypes,
    pub(crate) handlers: Vec<CppYonHandler>,
    methods: Vec<(CppYonErrorMethod, usize)>,
}

impl CppYonErrors {
    const FOLDER: &'static str = "server/controllers";
    const MARKER: &'static str = "ErrorHandler";
    const NO_METHOD: &'static str = "A C++ [[ErrorHandler]] class needs at least one [[HandlesError]] instance method, for example `[[HandlesError]] [[ResponseStatus(404)]] Problem missing(const OrderNotFound& error)`";

    pub(crate) fn read(root: &Path, controller: &CppYonController) -> Result<Self, Failure> {
        let types = CppYonErrorTypes::read(root)?;
        let handled = controller.handles_errors();
        let handlers = handled
            .then(|| Self::handlers(root))
            .transpose()?
            .unwrap_or_default();
        let methods = Self::methods(&types, &handlers)?;
        Ok(Self {
            types,
            handlers,
            methods,
        })
    }

    pub(crate) fn is_handler(file: &CppFile) -> bool {
        file.class
            .markers
            .iter()
            .any(|marker| marker.name == Self::MARKER)
    }

    pub(crate) fn responses(&self) -> Vec<ErrorResponse> {
        self.methods
            .iter()
            .map(|(method, _)| method.response())
            .collect()
    }

    pub(crate) fn type_source(&self) -> String {
        self.types.located()
    }

    pub(crate) fn handling(&self) -> String {
        let caught: String = self
            .methods
            .iter()
            .map(|(method, _)| Self::caught(method))
            .collect();
        format!(
            "    static tachyon::yon::YonAnswer handled(std::exception_ptr thrown, const tachyon::yon::YonFrame& frame) {{\n        (void)frame;\n        try {{\n            std::rethrow_exception(thrown);\n        }} catch (const tachyon::yon::YonArgumentException&) {{\n            throw;\n        }}{caught}\n        return tachyon::yon::YonAnswer::empty(500);\n    }}\n"
        )
    }

    fn caught(method: &CppYonErrorMethod) -> String {
        let request = [
            String::new(),
            String::from(", tachyon::yon::YonRequest::of(frame)"),
        ][usize::from(method.takes_request)]
        .clone();
        format!(
            " catch (::{kind}& error) {{\n            return tachyon::yon::YonAnswer::handled(tachyon::yon::YonNeeds::of<::{owner}>().{name}(error{request}), {status});\n        }}",
            kind = method.error_type,
            owner = method.owner,
            name = method.method,
            status = method.status,
        )
    }

    fn handlers(root: &Path) -> Result<Vec<CppYonHandler>, Failure> {
        let read = CppFolder::files(root, Self::FOLDER)?
            .iter()
            .filter(|path| !Self::generated(path))
            .map(|path| CppFolder::read(root, path))
            .map(|read| read.and_then(|(path, source)| Self::handler(path, source)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(read.into_iter().flatten().collect())
    }

    fn generated(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'))
    }

    fn handler(path: PathBuf, source: String) -> Result<Option<CppYonHandler>, Failure> {
        let file = CppFile::read(&path, &source)?;
        Self::is_handler(&file)
            .then(|| {
                CppMarkerCheck::check(&path, &file.class)?;
                Ok(CppYonHandler { path, file, source })
            })
            .transpose()
    }

    fn methods(
        types: &CppYonErrorTypes,
        handlers: &[CppYonHandler],
    ) -> Result<Vec<(CppYonErrorMethod, usize)>, Failure> {
        let mut methods = Vec::new();
        for handler in handlers {
            let class = &handler.file.class;
            let found = CppYonErrorMethod::all(&handler.path, &class.name, &class.members)?;
            let empty = CppFailure::coded(
                CppYonController::CODE,
                &handler.path,
                class.name_at,
                Self::NO_METHOD,
            );
            (!found.is_empty()).then_some(()).ok_or(empty)?;
            methods.extend(found);
        }
        Self::checked(types, methods)
    }

    fn checked(
        types: &CppYonErrorTypes,
        methods: Vec<CppYonErrorMethod>,
    ) -> Result<Vec<(CppYonErrorMethod, usize)>, Failure> {
        let mut seen = BTreeSet::new();
        let mut ordered = methods
            .into_iter()
            .map(|method| Self::placed(types, &mut seen, method))
            .collect::<Result<Vec<_>, _>>()?;
        ordered.sort_by_key(|(_, depth)| std::cmp::Reverse(*depth));
        Ok(ordered)
    }

    fn placed(
        types: &CppYonErrorTypes,
        seen: &mut BTreeSet<String>,
        method: CppYonErrorMethod,
    ) -> Result<(CppYonErrorMethod, usize), Failure> {
        let problem = format!(
            "{} is not a C++ error type. The first parameter of a [[HandlesError]] method names a class in server/errors or a standard error such as std::runtime_error",
            method.error_type
        );
        let depth = types.depth(&method.error_type).ok_or_else(|| {
            CppFailure::coded(CppYonController::CODE, &method.path, method.at, &problem)
        })?;
        let first = seen.insert(method.error_type.clone());
        let twice = format!(
            "Two [[HandlesError]] methods handle {}. Each error type needs one handler in the project",
            method.error_type
        );
        first.then_some(()).ok_or_else(|| {
            CppFailure::coded(CppYonController::CODE, &method.path, method.at, &twice)
        })?;
        Ok((method, depth))
    }
}
