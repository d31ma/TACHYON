use std::path::Path;

pub(crate) struct StylePath;

impl StylePath {
    const SHARED: &'static str = "/shared/";
    const RESERVED_SEGMENTS: [&'static str; 3] = ["", ".", ".."];

    pub(crate) fn validate(source: &str) -> Result<(), &'static str> {
        Self::require(
            source.starts_with(Self::SHARED),
            "Stylesheet and vendor declarations must name local /shared/ resources.",
        )?;
        Self::require(
            Self::plain_path(source),
            "Stylesheet paths must be canonical unescaped local paths without queries or fragments.",
        )?;
        Self::validate_segments(source)
    }

    fn validate_segments(source: &str) -> Result<(), &'static str> {
        Self::require(
            source[1..].split('/').all(Self::normal_segment),
            "Stylesheet paths cannot contain empty, dot or parent segments.",
        )
    }

    fn normal_segment(part: &str) -> bool {
        !Self::RESERVED_SEGMENTS.contains(&part)
    }

    fn plain_path(source: &str) -> bool {
        !source.contains(['\\', ':', '?', '#', '%']) && !source.chars().any(char::is_control)
    }

    pub(super) fn imported(parent: &str, source: &str) -> Result<String, &'static str> {
        Self::require(
            !source.starts_with("//"),
            "Remote and protocol-relative CSS imports are not allowed.",
        )?;
        let path = Self::joined(parent, source);
        let normalized = Self::normalize(&path)?;
        Self::validate(&normalized)?;
        Self::require(
            Path::new(&normalized)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("css")),
            "CSS imports must name local .css files.",
        )?;
        Ok(normalized)
    }

    fn joined(parent: &str, source: &str) -> String {
        if source.starts_with('/') {
            String::from(source)
        } else {
            format!(
                "{}/{source}",
                parent
                    .rsplit_once('/')
                    .map_or("", |(directory, _)| directory)
            )
        }
    }

    fn normalize(path: &str) -> Result<String, &'static str> {
        let mut parts = Vec::new();
        path.split('/')
            .try_for_each(|part| Self::add_part(part, &mut parts))?;
        Ok(format!("/{}", parts.join("/")))
    }

    fn add_part<'a>(part: &'a str, parts: &mut Vec<&'a str>) -> Result<(), &'static str> {
        match part {
            "" | "." => Ok(()),
            ".." => Self::remove_part(parts),
            _ => {
                parts.push(part);
                Ok(())
            }
        }
    }

    fn remove_part(parts: &mut Vec<&str>) -> Result<(), &'static str> {
        Self::require(
            parts.len() > 1,
            "CSS import escapes the captured shared asset root.",
        )?;
        parts.pop();
        Ok(())
    }

    fn require(valid: bool, message: &'static str) -> Result<(), &'static str> {
        valid.then_some(()).ok_or(message)
    }
}
