use crate::Failure;
use crate::annotation_layout;
use std::path::Path;

pub(super) struct CompositionCheck;

impl CompositionCheck {
    const CONTROLLERS: &'static str = "server/controllers";

    pub(super) fn is_composed(path: &Path, source: &str) -> bool {
        Self::test_answer(path, source)
            .or_else(|| Self::controller_answer(path, source))
            .or_else(|| Self::error_handler_answer(path, source))
            .or_else(|| Self::validation_answer(path, source))
            .or_else(|| Self::test_annotation_answer(path, source))
            .or_else(|| Self::error_source_answer(path))
            .or_else(|| Self::utility_answer(path))
            .unwrap_or_else(|| Self::annotation_answer(path, source))
    }

    fn test_answer(path: &Path, source: &str) -> Option<bool> {
        Self::test_environment(path).map(|environment| {
            crate::behavior::is_test_configuration(path, source, environment).unwrap_or(false)
        })
    }

    fn test_environment(path: &Path) -> Option<&'static str> {
        if path.starts_with("client/tests") {
            Some("tac")
        } else if path.starts_with("server/tests") {
            Some("yon")
        } else {
            None
        }
    }

    fn controller_answer(path: &Path, source: &str) -> Option<bool> {
        Self::is_controller(path).then(|| Self::is_routed_controller(path, source))
    }

    fn is_controller(path: &Path) -> bool {
        path.starts_with(Self::CONTROLLERS) && !Self::is_error_handler_name(path)
    }

    fn is_routed_controller(path: &Path, source: &str) -> bool {
        Self::is_directly_in_controllers(path)
            && annotation_layout::has_route_controller(path, source).unwrap_or(false)
    }

    fn error_handler_answer(path: &Path, source: &str) -> Option<bool> {
        let rejected = Self::is_directly_in_controllers(path)
            && !annotation_layout::has_error_handler(path, source).unwrap_or(false);
        rejected.then_some(false)
    }

    fn validation_answer(path: &Path, source: &str) -> Option<bool> {
        let rejected = annotation_layout::is_error_source(path)
            && annotation_layout::validate(path, source).is_err();
        rejected.then_some(false)
    }

    fn test_annotation_answer(path: &Path, source: &str) -> Option<bool> {
        let excluded = annotation_layout::has_test_annotation(path, source).unwrap_or(true);
        excluded.then_some(false)
    }

    fn error_source_answer(path: &Path) -> Option<bool> {
        annotation_layout::is_error_source(path).then_some(true)
    }

    fn utility_answer(path: &Path) -> Option<bool> {
        annotation_layout::UtilityLayout::is_utility(path).then_some(true)
    }

    fn annotation_answer(path: &Path, source: &str) -> bool {
        Self::has_role_or_annotation(path, source).unwrap_or(false)
    }

    fn has_role_or_annotation(path: &Path, source: &str) -> Result<bool, Failure> {
        Ok(annotation_layout::has_primary_role(path, source)?
            || annotation_layout::has_framework_annotation(path, source)?)
    }

    fn is_directly_in_controllers(path: &Path) -> bool {
        path.parent() == Some(Path::new(Self::CONTROLLERS))
    }

    fn is_error_handler_name(path: &Path) -> bool {
        path.file_stem()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("ErrorHandler"))
    }
}
