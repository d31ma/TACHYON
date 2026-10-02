use crate::Failure;
use crate::project::NativeCompanion;
use crate::template::ServiceDefinition;
use std::io::Read as _;
use std::path::Path;
use tachyon_contracts::NativeTarget;

pub(super) struct NativeApplicationInputs;

impl NativeApplicationInputs {
    const MAX_SOURCE: u64 = 1024 * 1024;

    pub(super) fn provider(
        root: &Path,
        service: &ServiceDefinition,
        target: NativeTarget,
        exposed: bool,
    ) -> Result<bool, Failure> {
        Self::rust_path(service)
            .map(|path| Self::captured(root, path, (service, target, exposed)))
            .transpose()
            .map(|provider| provider.unwrap_or(false))
    }

    fn rust_path(service: &ServiceDefinition) -> Option<&Path> {
        service
            .native_companions()
            .iter()
            .find(|(language, _)| *language == NativeCompanion::Rust)
            .map(|(_, path)| path.as_path())
    }

    fn captured(
        root: &Path,
        path: &Path,
        selection: (&ServiceDefinition, NativeTarget, bool),
    ) -> Result<bool, Failure> {
        let relative = path.strip_prefix(root).unwrap_or(path);
        let source = Self::read(path)?;
        let roles = crate::behavior::ServiceRoles::read(relative, &source)?;
        roles
            .application
            .map(|at| Self::selected(relative, at, selection))
            .transpose()
            .map(|provider| provider.unwrap_or(false))
    }

    fn read(path: &Path) -> Result<String, Failure> {
        let file = std::fs::File::open(path)
            .map_err(|error| Self::failure(path, 0, &error.to_string()))?;
        let mut source = String::new();
        file.take(Self::MAX_SOURCE + 1)
            .read_to_string(&mut source)
            .map_err(|error| Self::failure(path, 0, &error.to_string()))?;
        Self::bounded(path, source)
    }

    fn bounded(path: &Path, source: String) -> Result<String, Failure> {
        let valid = [
            source.len() as u64 <= Self::MAX_SOURCE,
            !source.contains('\0'),
        ] == [true; 2];
        valid.then_some(source).ok_or_else(|| {
            Self::failure(
                path,
                0,
                "Application provider source must be bounded UTF-8 without NUL, at most 1 MiB.",
            )
        })
    }

    fn selected(
        path: &Path,
        at: usize,
        selection: (&ServiceDefinition, NativeTarget, bool),
    ) -> Result<bool, Failure> {
        let (service, target, exposed) = selection;
        crate::behavior::NativeApplicationLifetime::target(path, Some(at), Some(target))?;
        Self::require_not_exposed(path, at, exposed)?;
        let languages = service
            .native_companions()
            .iter()
            .map(|(language, _)| *language)
            .collect::<Vec<_>>();
        let selected = NativeCompanion::most_specific(&languages, target);
        (selected == Some(NativeCompanion::Rust))
            .then_some(true)
            .ok_or_else(|| Self::failure(path, at, "Application lifetime requires the selected native Rust companion; another platform variant cannot replace its application provider."))
    }

    fn require_not_exposed(path: &Path, at: usize, exposed: bool) -> Result<(), Failure> {
        (!exposed).then_some(()).ok_or_else(|| Self::failure(path, at,
            "An application Service is a native provider, not a directly view-bound Service. Inject Arc<ProviderService> into a fresh page Service."))
    }

    fn failure(path: &Path, at: usize, message: &str) -> Failure {
        Failure::one(crate::failure::diagnostic(
            1406,
            message,
            None,
            crate::failure::source_span(&path.to_string_lossy(), at, at + "Lifetime".len()),
        ))
    }
}
