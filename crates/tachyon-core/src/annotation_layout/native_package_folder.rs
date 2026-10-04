use super::NativeModuleVendor;
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
    pub(crate) const PROGRAM_FOLDER: &'static str = "yonhandler";
    const CODE: u16 = 1310;
    const GO: &'static str = "go";
    const MODULE: &'static str = "module";
    const ALLOWED_DIRECTIVES: [&'static str; 4] = ["module", "go", "toolchain", "require"];
    const BLOCK_START: &'static str = "(";
    const BLOCK_END: &'static str = ")";
    const WORD_ENDS: [char; 3] = [' ', '\t', '('];
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

    pub(crate) fn directives(module_file: &str) -> Vec<&str> {
        module_file
            .lines()
            .map(Self::without_comment)
            .scan(false, |in_block, line| {
                Some(Self::directive_of_line(in_block, line))
            })
            .flatten()
            .collect()
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
        let vendored = NativeModuleVendor::contains(path);
        let reserved = path.starts_with(Path::new(Self::ROOT).join(Self::PROGRAM_FOLDER));
        let root_module = path == Path::new(Self::ROOT).join(Self::MODULE_FILE);
        let module = path.file_name().and_then(OsStr::to_str) == Some(Self::MODULE_FILE);
        let go = Self::extension(path) == Self::GO;
        match (vendored, reserved, root_module, module, go) {
            (true, _, _, _, _) => Ok(()),
            (false, true, _, _, _) => Self::refuse_reserved_folder(path),
            (false, false, true, _, _) => Self::check_module_file(path, bytes),
            (false, false, false, true, _) => Self::refuse_nested_module(path),
            (false, false, false, false, true) => Self::check_go_file(path, bytes),
            (false, false, false, false, false) => Self::check_other_file(path),
        }
    }

    fn directive_of_line<'a>(in_block: &mut bool, line: &'a str) -> Option<&'a str> {
        let (still_in_block, directive) = if *in_block {
            (line != Self::BLOCK_END, None)
        } else {
            let word = line
                .split(Self::WORD_ENDS)
                .next()
                .filter(|word| !word.is_empty());
            (line.ends_with(Self::BLOCK_START), word)
        };
        *in_block = still_in_block;
        directive
    }

    fn refuse_reserved_folder(path: &Path) -> Result<(), Failure> {
        Err(Self::failure(
            path,
            "server/packages/yonhandler/ is reserved: Tachyon builds each Go handler program in that folder of the module; give the package another name",
        ))
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
        let unsupported = Self::directives(&text)
            .into_iter()
            .find(|directive| !Self::ALLOWED_DIRECTIVES.contains(directive));
        unsupported.map_or(Ok(()), |directive| {
            Err(Self::failure(path, &format!("server/packages/go.mod may hold only module, go, toolchain and require lines; found `{directive}`. Required third-party modules come from the vendor/ folder that `go mod vendor` writes")))
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
