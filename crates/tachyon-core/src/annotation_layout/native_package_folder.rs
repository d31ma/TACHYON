use crate::Failure;
use crate::annotation_rules::AnnotationRules;
use crate::failure::{diagnostic, source_span};
use std::ffi::OsStr;
use std::path::Path;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) struct NativePackageFolder;

impl NativePackageFolder {
    pub(crate) const ROOT: &'static str = "server/packages";
    pub(crate) const MODULE_FILE: &'static str = "go.mod";
    const CODE: u16 = 1310;
    const GO: &'static str = "go";
    const MODULE: &'static str = "module";
    const ALLOWED_DIRECTIVES: [&'static str; 3] = ["module", "go", "toolchain"];
    const HELP: &'static str = "server/packages/ holds one native Go module: server/packages/go.mod names the module, and its plain Go packages are imported by controllers and services with that module path (ADR 0095). Tachyon roles stay in their role folders.";

    pub(crate) fn contains(path: &Path) -> bool {
        path.starts_with(Self::ROOT)
    }

    pub(crate) fn module_path(module_file: &str) -> Option<String> {
        module_file
            .lines()
            .map(Self::without_comment)
            .find_map(|line| Self::directive_value(line, Self::MODULE))
            .map(|path| path.trim_matches('"').to_owned())
            .filter(|path| !path.is_empty())
    }

    pub(crate) fn check_file(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        Self::check_by_kind(path, bytes)
    }

    pub(crate) fn check_module<'a>(paths: impl Iterator<Item = &'a Path>) -> Result<(), Failure> {
        let module_file = Path::new(Self::ROOT).join(Self::MODULE_FILE);
        let native = paths
            .filter(|path| Self::contains(path))
            .collect::<Vec<_>>();
        let missing = !native.contains(&module_file.as_path());
        native.first().filter(|_| missing).map_or(Ok(()), |path| {
            Err(Self::failure(
                path,
                "server/packages/ has no go.mod; add server/packages/go.mod with a module line",
            ))
        })
    }

    fn check_by_kind(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        let root_module = path == Path::new(Self::ROOT).join(Self::MODULE_FILE);
        let module = path.file_name().and_then(OsStr::to_str) == Some(Self::MODULE_FILE);
        let go = Self::extension(path) == Self::GO;
        match (root_module, module, go) {
            (true, _, _) => Self::check_module_file(path, bytes),
            (false, true, _) => Self::refuse_nested_module(path),
            (false, false, true) => Self::check_go_file(path, bytes),
            (false, false, false) => Self::check_other_file(path),
        }
    }

    fn extension(path: &Path) -> &str {
        path.extension().and_then(OsStr::to_str).unwrap_or("")
    }

    fn refuse_nested_module(path: &Path) -> Result<(), Failure> {
        Err(Self::failure(
            path,
            "A native Go module cannot hold another go.mod; server/packages/go.mod is the only module file",
        ))
    }

    fn check_other_file(path: &Path) -> Result<(), Failure> {
        let extension = Self::extension(path);
        let source = AnnotationRules::LANGUAGES.contains(&extension);
        source.then(|| Self::failure(path, &format!("server/packages/ holds native Go code; a .{extension} file belongs in a role folder or in server/shared"))).map_or(Ok(()), Err)
    }

    fn check_module_file(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        let text = Self::text(path, bytes)?;
        Self::module_path(&text)
            .ok_or_else(|| Self::failure(path, "server/packages/go.mod has no module line"))?;
        let unsupported = text
            .lines()
            .map(Self::without_comment)
            .filter_map(|line| line.split_whitespace().next())
            .find(|directive| !Self::ALLOWED_DIRECTIVES.contains(directive));
        unsupported.map_or(Ok(()), |directive| {
            Err(Self::failure(path, &format!("server/packages/go.mod may hold only module, go and toolchain lines; found `{directive}`. Yon builds Go offline, so a native module cannot require other modules")))
        })
    }

    fn check_go_file(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        let text = Self::text(path, bytes)?;
        let annotated = super::has_framework_annotation(path, crate::without_bom(&text))?;
        annotated
            .then(|| Self::failure(path, "Native Go code in server/packages/ cannot carry a Tachyon annotation; an annotated type belongs in its role folder"))
            .map_or(Ok(()), Err)
    }

    fn text(path: &Path, bytes: &[u8]) -> Result<String, Failure> {
        String::from_utf8(bytes.to_vec())
            .map_err(|_| Self::failure(path, "A Go source or module file must be UTF-8"))
    }

    fn without_comment(line: &str) -> &str {
        line.split("//").next().unwrap_or("").trim()
    }

    fn directive_value<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
        let value = line.strip_prefix(directive)?;
        value.starts_with(char::is_whitespace).then(|| value.trim())
    }

    fn failure(path: &Path, message: &str) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            format!("'{portable}': {message}."),
            Some(String::from(Self::HELP)),
            source_span(&portable, 0, 0),
        ))
    }
}
