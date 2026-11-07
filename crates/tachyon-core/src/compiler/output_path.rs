use super::build_stage::BuildStage;
use crate::Failure;
use crate::failure::diagnostic;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub(crate) struct OutputPath;

impl OutputPath {
    const RESERVED: [&'static str; 3] = ["client", "server", ".git"];
    const ABSOLUTE_SYMLINK_HELP: &'static str = "Choose a regular output directory.";
    const RELATIVE_SYMLINK_HELP: &'static str = "Choose a regular project-relative directory.";

    pub(crate) fn resolve(root: &Path, path: &Path) -> Result<PathBuf, Failure> {
        Self::resolve_by_kind(root, path)
    }

    fn resolve_by_kind(root: &Path, path: &Path) -> Result<PathBuf, Failure> {
        if path.is_absolute() {
            Self::resolve_absolute(root, path)
        } else {
            Self::resolve_relative(root, path)
        }
    }

    fn resolve_relative(root: &Path, path: &Path) -> Result<PathBuf, Failure> {
        let relative = Self::validate(path)?;
        Self::assert_safe(root, relative)?;
        Ok(root.join(relative))
    }

    fn resolve_absolute(root: &Path, path: &Path) -> Result<PathBuf, Failure> {
        let normalized_root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let normalized = Self::normalize_absolute(path)?;
        Self::refuse_reserved(&normalized, &normalized_root, path)?;
        Self::check_absolute_segments(path, &normalized)?;
        Ok(normalized)
    }

    fn validate(path: &Path) -> Result<&Path, Failure> {
        if !Self::is_safe_relative(path) {
            return Err(Self::invalid(path));
        }
        Ok(path)
    }

    fn is_safe_relative(path: &Path) -> bool {
        let mut components = path.components();
        let first = components.next();
        let checks = [
            !path.as_os_str().is_empty(),
            !path.is_absolute(),
            Self::is_allowed_first(first),
            components.all(|component| Self::normal_segment(component).is_some()),
        ];
        checks.iter().all(|passed| *passed)
    }

    fn is_allowed_first(first: Option<Component<'_>>) -> bool {
        match first {
            Some(Component::Normal(name)) => {
                !Self::RESERVED.iter().any(|reserved| name == *reserved)
            }
            _ => false,
        }
    }

    fn normal_segment(component: Component<'_>) -> Option<&OsStr> {
        match component {
            Component::Normal(segment) => Some(segment),
            _ => None,
        }
    }

    fn refuse_reserved(normalized: &Path, root: &Path, path: &Path) -> Result<(), Failure> {
        if normalized == root
            || normalized == Path::new("/")
            || normalized.starts_with(root.join("client"))
            || normalized.starts_with(root.join("server"))
            || normalized.starts_with(root.join(".git"))
        {
            return Err(Self::invalid(path));
        }
        Ok(())
    }

    fn check_absolute_segments(path: &Path, normalized: &Path) -> Result<(), Failure> {
        let mut current = PathBuf::new();
        for component in normalized.components() {
            Self::check_component(&mut current, component, path, normalized)?;
        }
        Ok(())
    }

    fn check_component(
        current: &mut PathBuf,
        component: Component<'_>,
        path: &Path,
        normalized: &Path,
    ) -> Result<(), Failure> {
        match component {
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
                Ok(())
            }
            Component::RootDir => {
                Self::push_and_check(current, component.as_os_str(), path, normalized)
            }
            Component::Normal(segment) => Self::push_and_check(current, segment, path, normalized),
            Component::CurDir | Component::ParentDir => Err(Self::invalid(path)),
        }
    }

    fn push_and_check(
        current: &mut PathBuf,
        segment: &OsStr,
        path: &Path,
        normalized: &Path,
    ) -> Result<(), Failure> {
        current.push(segment);
        let is_target = current == normalized;
        Self::check_segment(current, is_target, path, Self::ABSOLUTE_SYMLINK_HELP)
    }

    fn check_segment(
        current: &Path,
        is_target: bool,
        shown_path: &Path,
        symlink_help: &str,
    ) -> Result<(), Failure> {
        match fs::symlink_metadata(current) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err(Failure::one(diagnostic(
                1202,
                format!("Output path '{}' contains a symlink.", shown_path.display()),
                Some(String::from(symlink_help)),
                None,
            ))),
            Ok(metadata) if is_target && !metadata.is_dir() => Err(Failure::one(diagnostic(
                1202,
                format!("Output path '{}' is not a directory.", shown_path.display()),
                None,
                None,
            ))),
            Ok(_) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(BuildStage::output_error(current, &error)),
        }
    }

    fn normalize_absolute(path: &Path) -> Result<PathBuf, Failure> {
        let (existing, missing) = Self::existing_ancestor(path)?;
        Self::refuse_symlinked_target(path, existing)?;
        let mut normalized = fs::canonicalize(existing)
            .map_err(|error| BuildStage::output_error(existing, &error))?;
        normalized.extend(missing.iter().rev());
        Ok(normalized)
    }

    fn existing_ancestor(path: &Path) -> Result<(&Path, Vec<OsString>), Failure> {
        let mut current = path;
        let mut missing = Vec::new();
        while Self::is_missing(current)? {
            let segment = current.file_name().ok_or_else(|| Self::invalid(path))?;
            missing.push(segment.to_os_string());
            current = current.parent().ok_or_else(|| Self::invalid(path))?;
        }
        Ok((current, missing))
    }

    fn is_missing(current: &Path) -> Result<bool, Failure> {
        match fs::symlink_metadata(current) {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(error) => Err(BuildStage::output_error(current, &error)),
        }
    }

    fn refuse_symlinked_target(path: &Path, existing: &Path) -> Result<(), Failure> {
        if existing == path
            && fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(Failure::one(diagnostic(
                1202,
                format!("Output path '{}' is a symlink.", path.display()),
                Some(String::from(Self::ABSOLUTE_SYMLINK_HELP)),
                None,
            )));
        }
        Ok(())
    }

    fn invalid(path: &Path) -> Failure {
        Failure::one(diagnostic(
            1202,
            format!("Output directory '{}' is not a safe path.", path.display()),
            Some(String::from(
                "Use a regular directory outside client/, server/, and .git/.",
            )),
            None,
        ))
    }

    fn assert_safe(root: &Path, relative: &Path) -> Result<(), Failure> {
        let target = root.join(relative);
        let mut current = root.to_path_buf();
        for component in relative.components() {
            Self::check_relative_component(&mut current, component, &target, relative)?;
        }
        Ok(())
    }

    fn check_relative_component(
        current: &mut PathBuf,
        component: Component<'_>,
        target: &Path,
        relative: &Path,
    ) -> Result<(), Failure> {
        let segment = Self::normal_segment(component).ok_or_else(|| Self::invalid(relative))?;
        current.push(segment);
        let is_target = current == target;
        Self::check_segment(current, is_target, relative, Self::RELATIVE_SYMLINK_HELP)
    }
}
