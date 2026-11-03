use std::path::{Component, MAIN_SEPARATOR, Path};

pub(crate) struct PortablePath;

impl PortablePath {
    pub(crate) fn of(path: &Path) -> String {
        let segments: Vec<&str> = path.components().filter_map(Self::normal_text).collect();
        segments.join("/")
    }

    pub(crate) fn file(path: &Path) -> String {
        Self::file_text(&path.to_string_lossy())
    }

    pub(crate) fn file_text(file: &str) -> String {
        Self::with_forward_slashes(file, MAIN_SEPARATOR)
    }

    pub(crate) fn tool_output(text: &str) -> String {
        Self::tool_output_with(text, MAIN_SEPARATOR)
    }

    pub(super) fn route_key(route: &str) -> String {
        Self::key_for_route(route)
    }

    fn normal_text(component: Component<'_>) -> Option<&str> {
        match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        }
    }

    fn with_forward_slashes(text: &str, separator: char) -> String {
        text.replace(separator, "/")
    }

    fn tool_output_with(text: &str, separator: char) -> String {
        text.split_inclusive('\n')
            .map(|line| Self::tool_line(line, separator))
            .collect()
    }

    fn tool_line(line: &str, separator: char) -> String {
        let end = Self::location_end(line);
        let location = Self::with_forward_slashes(&line[..end], separator);
        location + &line[end..]
    }

    fn location_end(line: &str) -> usize {
        line.match_indices(':')
            .map(|(at, _)| at)
            .find(|at| line[at + 1..].starts_with(|next: char| next.is_ascii_digit()))
            .filter(|at| !line[..*at].contains(char::is_whitespace))
            .unwrap_or(0)
    }

    fn key_for_route(route: &str) -> String {
        if route == "/" {
            return String::from("root");
        }
        route.trim_start_matches('/').replace('/', "__")
    }
}

#[cfg(test)]
mod tests {
    use super::PortablePath;
    use std::path::Path;

    #[test]
    fn a_native_relative_file_is_named_with_forward_slashes() {
        let native = Path::new("client")
            .join("delegates")
            .join("StepDelegate.cs");
        assert_eq!(
            PortablePath::file(&native),
            "client/delegates/StepDelegate.cs"
        );
    }

    #[test]
    fn only_the_native_separator_becomes_a_forward_slash() {
        let windows = PortablePath::with_forward_slashes(r"client\pages\tac.html", '\\');
        assert_eq!(windows, "client/pages/tac.html");
        let unix = PortablePath::with_forward_slashes(r"client\pages\tac.html", '/');
        assert_eq!(unix, r"client\pages\tac.html");
    }

    #[test]
    fn tool_output_names_its_source_locations_with_forward_slashes() {
        let output = "# tachyonservices/service0\nservice0\\client\\services\\Broken.go:5: undefined: missing\n.\\server\\models\\Guide.go:3: bad escape \"\\q\"";
        assert_eq!(
            PortablePath::tool_output_with(output, '\\'),
            "# tachyonservices/service0\nservice0/client/services/Broken.go:5: undefined: missing\n./server/models/Guide.go:3: bad escape \"\\q\""
        );
    }

    #[test]
    fn tool_output_without_a_location_is_unchanged() {
        for text in [
            "Native tool 'go' failed with status exit code: 1: # a\\b",
            "no detail",
            "",
            "C:\\work\\x.go:4: boom",
        ] {
            assert_eq!(PortablePath::tool_output_with(text, '/'), text);
        }
        assert_eq!(
            PortablePath::tool_output_with("Native tool 'go' failed: 1: # a\\b", '\\'),
            "Native tool 'go' failed: 1: # a\\b"
        );
        assert_eq!(
            PortablePath::tool_output_with("C:\\work\\x.go:4: boom", '\\'),
            "C:/work/x.go:4: boom"
        );
    }
}
