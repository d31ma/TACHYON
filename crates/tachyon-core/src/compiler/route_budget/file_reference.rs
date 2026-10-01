pub(super) struct FileReference;

impl FileReference {
    const QUOTES: [char; 3] = ['"', '\'', '`'];
    const QUERY_MARKS: [char; 2] = ['?', '#'];
    const READ_EXTENSIONS: [&'static str; 4] = [".html", ".js", ".mjs", ".json"];
    const REFERENCE_EXTENSIONS: [&'static str; 5] = [".js", ".mjs", ".wasm", ".json", ".zip"];
    const SCHEME_MARK: &'static str = "://";
    const SAME_SCHEME_MARK: &'static str = "//";
    const SEPARATOR: char = '/';
    const CURRENT_FOLDER: &'static str = ".";
    const PARENT_FOLDER: &'static str = "..";

    pub(super) fn resolved(from: &str, bytes: &[u8]) -> Vec<String> {
        let text = Self::readable_text(from, bytes);
        text.split(Self::QUOTES)
            .filter(|part| Self::is_reference(part))
            .map(|part| Self::resolve(from, part))
            .collect()
    }

    fn readable_text(from: &str, bytes: &[u8]) -> String {
        if Self::has_extension(from, &Self::READ_EXTENSIONS) {
            String::from_utf8_lossy(bytes).into_owned()
        } else {
            String::new()
        }
    }

    fn is_reference(part: &str) -> bool {
        let path = Self::without_query(part);
        let local = !part.contains(Self::SCHEME_MARK) & !part.starts_with(Self::SAME_SCHEME_MARK);
        let plain = !part.contains(char::is_whitespace);
        local & plain & Self::has_extension(path, &Self::REFERENCE_EXTENSIONS)
    }

    fn has_extension(path: &str, extensions: &[&str]) -> bool {
        extensions.iter().any(|extension| path.ends_with(extension))
    }

    fn without_query(part: &str) -> &str {
        part.split(Self::QUERY_MARKS).next().unwrap_or_default()
    }

    fn resolve(from: &str, reference: &str) -> String {
        let path = Self::without_query(reference);
        let joined = format!("{}{path}", Self::folder_of(from, path));
        Self::normal(&joined)
    }

    fn folder_of(from: &str, path: &str) -> String {
        if path.starts_with(Self::SEPARATOR) {
            String::new()
        } else {
            let end = from.rfind(Self::SEPARATOR).map_or(0, |index| index + 1);
            String::from(&from[..end])
        }
    }

    fn normal(path: &str) -> String {
        let mut segments = Vec::new();
        for segment in path.split(Self::SEPARATOR) {
            Self::step(&mut segments, segment);
        }
        segments.join("/")
    }

    fn step<'a>(segments: &mut Vec<&'a str>, segment: &'a str) {
        match segment {
            "" | Self::CURRENT_FOLDER => {}
            Self::PARENT_FOLDER => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }
}
