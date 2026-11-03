pub(crate) struct RoutePath;

impl RoutePath {
    const MAX_PATH_BYTES: usize = 1_024;

    pub(crate) fn pattern(base: &str, path: &str) -> Result<String, String> {
        Self::check_start(base)?;
        Self::check_start(path)?;
        let joined = format!("{base}{path}");
        Self::check_length(&joined)?;
        Self::pattern_of(&joined)
    }

    pub(crate) fn template(pattern: &str) -> String {
        let segments: Vec<String> = pattern.split('/').map(Self::template_segment).collect();
        segments.join("/")
    }

    pub(crate) fn valid_parameter_name(value: &str) -> bool {
        let checks = [
            (1..=64).contains(&value.len()),
            Self::starts_with_lowercase(value),
            value.bytes().all(Self::parameter_byte),
        ];
        checks.iter().all(|passed| *passed)
    }

    pub(crate) fn valid_static_segment(segment: &str) -> bool {
        let checks = [
            !segment.is_empty(),
            !segment.starts_with('-'),
            !segment.ends_with('-'),
            !segment.contains("--"),
            segment.bytes().all(Self::static_byte),
        ];
        checks.iter().all(|passed| *passed)
    }

    fn template_segment(segment: &str) -> String {
        match segment.strip_prefix('_') {
            Some(name) => format!("{{{name}}}"),
            None => segment.to_owned(),
        }
    }

    fn starts_with_lowercase(value: &str) -> bool {
        value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
    }

    fn parameter_byte(byte: u8) -> bool {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
    }

    fn static_byte(byte: u8) -> bool {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
    }

    fn check_start(path: &str) -> Result<(), String> {
        if path.is_empty() || path.starts_with('/') {
            return Ok(());
        }
        Err(format!(
            "The route path '{path}' must start with '/', for example \"/orders\"."
        ))
    }

    fn check_length(joined: &str) -> Result<(), String> {
        if joined.len() > Self::MAX_PATH_BYTES {
            return Err(String::from("A route path may be at most 1024 bytes long."));
        }
        Ok(())
    }

    fn pattern_of(joined: &str) -> Result<String, String> {
        if joined.is_empty() || joined == "/" {
            return Ok(String::from("/"));
        }
        Self::internal_pattern(joined)
    }

    fn internal_pattern(joined: &str) -> Result<String, String> {
        let mut names = Vec::new();
        let segments = joined[1..]
            .split('/')
            .map(|segment| Self::internal_segment(segment, joined, &mut names))
            .collect::<Result<Vec<String>, String>>()?;
        Ok(format!("/{}", segments.join("/")))
    }

    fn internal_segment(
        segment: &str,
        path: &str,
        names: &mut Vec<String>,
    ) -> Result<String, String> {
        Self::check_part_not_empty(segment, path)?;
        Self::braced_name(segment).map_or_else(
            || Self::static_part(segment, path),
            |name| Self::parameter_part(name, segment, path, names),
        )
    }

    fn check_part_not_empty(segment: &str, path: &str) -> Result<(), String> {
        if segment.is_empty() {
            return Err(format!(
                "The route path '{path}' has an empty part. Remove the extra '/'."
            ));
        }
        Ok(())
    }

    fn braced_name(segment: &str) -> Option<&str> {
        segment
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
    }

    fn static_part(segment: &str, path: &str) -> Result<String, String> {
        if Self::valid_static_segment(segment) {
            return Ok(segment.to_owned());
        }
        Err(format!(
            "The part '{segment}' of the route path '{path}' may use only lowercase letters, digits and single hyphens. A path variable is written in braces, like {{id}}."
        ))
    }

    fn parameter_part(
        name: &str,
        segment: &str,
        path: &str,
        names: &mut Vec<String>,
    ) -> Result<String, String> {
        Self::check_parameter_name(name, segment, path)?;
        Self::check_unique_name(name, path, names)?;
        names.push(name.to_owned());
        Ok(format!("_{name}"))
    }

    fn check_parameter_name(name: &str, segment: &str, path: &str) -> Result<(), String> {
        if !Self::valid_parameter_name(name) {
            return Err(format!(
                "The path variable '{segment}' in '{path}' needs a name of lowercase letters, digits and underscores, for example {{id}}."
            ));
        }
        Ok(())
    }

    fn check_unique_name(name: &str, path: &str, names: &[String]) -> Result<(), String> {
        if names.iter().any(|earlier| earlier == name) {
            return Err(format!(
                "The path variable '{{{name}}}' appears twice in '{path}'. Give each path variable its own name."
            ));
        }
        Ok(())
    }
}
