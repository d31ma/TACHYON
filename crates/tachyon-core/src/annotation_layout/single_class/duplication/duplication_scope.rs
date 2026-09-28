use std::path::Path;

pub(in super::super) struct DuplicationScope;

impl DuplicationScope {
    const EXTENSIONS: [&'static str; 13] = [
        "js", "ts", "py", "rb", "php", "java", "kt", "cs", "fs", "go", "rs", "dart", "swift",
    ];
    const SKIPPED_FOLDERS: [&'static str; 5] = ["tests", "models", "entities", "vendor", "assets"];
    const GENERATED_MARKS: [&'static str; 5] =
        [".d.ts", ".min.", ".generated.", "_generated.", ".g."];

    pub(in super::super) fn covers(path: &Path) -> bool {
        Self::is_code(path) & !Self::in_skipped_folder(path) & !Self::is_generated(path)
    }

    fn is_code(path: &Path) -> bool {
        path.extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| Self::EXTENSIONS.contains(&extension))
    }

    fn in_skipped_folder(path: &Path) -> bool {
        path.components()
            .filter_map(|part| part.as_os_str().to_str())
            .any(|part| Self::SKIPPED_FOLDERS.contains(&part))
    }

    fn is_generated(path: &Path) -> bool {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        Self::GENERATED_MARKS.iter().any(|mark| name.contains(mark))
    }
}
