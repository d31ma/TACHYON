use crate::Failure;
use crate::compiler::PortablePath;
use crate::failure::{diagnostic, source_span};
use crate::routing::RoutePath;
use std::path::{Component, Path};

pub(super) struct ViewRoute;

impl ViewRoute {
    pub(super) fn of(relative: &Path, source_root: &Path) -> Result<String, Failure> {
        let beneath_root = relative
            .strip_prefix(source_root)
            .map_err(|_| Self::escaped(relative))?;
        let parent = beneath_root.parent().unwrap_or(Path::new(""));
        let segments = Self::segments(parent, relative)?;
        Ok(format!("/{}", segments.join("/")))
    }

    fn segments<'a>(parent: &'a Path, relative: &Path) -> Result<Vec<&'a str>, Failure> {
        let mut segments = Vec::new();
        for component in parent.components() {
            let segment = Self::segment(component, relative)?;
            Self::check_segment(segment, &segments, relative)?;
            segments.push(segment);
        }
        Ok(segments)
    }

    fn segment<'a>(component: Component<'a>, relative: &Path) -> Result<&'a str, Failure> {
        let Component::Normal(segment) = component else {
            return Err(Self::invalid_route(relative));
        };
        segment
            .to_str()
            .ok_or_else(|| Self::invalid_route(relative))
    }

    fn check_segment(segment: &str, earlier: &[&str], relative: &Path) -> Result<(), Failure> {
        match segment.strip_prefix('_') {
            Some(parameter) if !RoutePath::valid_parameter_name(parameter) => {
                Err(Self::invalid_parameter(segment, relative))
            }
            Some(parameter) if earlier.contains(&segment) => {
                Err(Self::repeated_parameter(parameter, relative))
            }
            None if !RoutePath::valid_static_segment(segment) => Err(Self::invalid_route(relative)),
            _ => Ok(()),
        }
    }

    fn escaped(relative: &Path) -> Failure {
        Failure::one(diagnostic(
            1004,
            format!(
                "Source '{}' escaped its source root.",
                PortablePath::of(relative)
            ),
            None,
            None,
        ))
    }

    fn invalid_parameter(segment: &str, relative: &Path) -> Failure {
        let path = PortablePath::of(relative);
        Failure::one(diagnostic(
            1006,
            format!("Dynamic route segment '{segment}' has an invalid name."),
            Some(String::from(
                "Use `_name` where name is lowercase alphanumeric with underscores.",
            )),
            source_span(&path, 0, path.len()),
        ))
    }

    fn repeated_parameter(parameter: &str, relative: &Path) -> Failure {
        let path = PortablePath::of(relative);
        Failure::one(diagnostic(
            1006,
            format!("Dynamic route segment '{parameter}' is declared twice."),
            Some(String::from(
                "Give each dynamic segment in one route a unique name.",
            )),
            source_span(&path, 0, path.len()),
        ))
    }

    fn invalid_route(relative: &Path) -> Failure {
        let path = PortablePath::of(relative);
        Failure::one(diagnostic(
            1005,
            format!("View source '{path}' contains an invalid static route segment."),
            Some(String::from(
                "Use lowercase ASCII letters, digits, and single internal hyphens.",
            )),
            source_span(&path, 0, path.len()),
        ))
    }
}
