use super::{Declaration, Failure, Input, LuaView, Path, Role, primary_role};
use crate::annotation_rules::{AnnotationRules, Realm};
use crate::failure::{diagnostic, source_span};

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) struct FolderRoleCheck;

impl FolderRoleCheck {
    pub(crate) const CODE: u16 = 1363;
    const NATIVE_PACKAGE_HELP: &'static str =
        ", or a native Go package to server/packages/ (ADR 0095)";
    const FOLDERS: [(&'static str, &'static str); 9] = [
        ("services", "Service"),
        ("stores", "Store"),
        ("remotes", "HTTPClient"),
        ("clients", "HTTPClient"),
        ("setups", "Setup"),
        ("entities", "Entity"),
        ("delegates", "Delegate"),
        ("converters", "Converter"),
        ("controllers", "RequestRoute"),
    ];

    pub(crate) fn check(path: &Path, source: &str) -> Result<(), Failure> {
        LuaView::either(path, source, Self::check_source)
    }

    pub(crate) fn is_candidate(path: &Path, source: &str) -> Result<bool, Failure> {
        LuaView::either(path, source, |path, source| {
            let input = Input::new(path, source);
            Ok(Self::has_concrete_type(
                input.extension,
                &input.masked_code()?,
            ))
        })
    }

    pub(super) fn check_source(path: &Path, source: &str) -> Result<(), Failure> {
        super::reject_renamed_annotations(path, source)?;
        let expected = Self::expected(path);
        expected.map_or(Ok(()), |(realm, folder, marker)| {
            Self::check_registered(path, source, realm, folder, marker)
        })
    }

    fn environment(realm: &str) -> &'static str {
        match realm {
            "client" => "tac",
            _ => "yon",
        }
    }

    fn check_registered(
        path: &Path,
        source: &str,
        realm: &str,
        folder: &str,
        marker: &str,
    ) -> Result<(), Failure> {
        let input = Input::new(path, source);
        let code = input.masked_code()?;
        let concrete = Self::has_concrete_type(input.extension, &code);
        let registered = primary_role(path, source)?;
        Self::check_suffix(path, source, concrete)?;
        let valid = [!concrete, registered].contains(&true);
        valid
            .then_some(())
            .ok_or_else(|| Self::failure(&input, realm, folder, marker))
    }

    fn check_suffix(path: &Path, source: &str, concrete: bool) -> Result<(), Failure> {
        let mismatch = concrete
            .then(|| Self::misplaced_suffix(path, source))
            .transpose()?
            .flatten();
        mismatch.map_or(Ok(()), |role| {
            let input = Input::new(path, source);
            Err(input.failure(0, "A role class suffix must match its realm role folder", format!("Move the class to {}/{} and mark it with {} (ADR0091). Nested organization is allowed; a nested folder cannot hide a Service class.", Self::realm(path), role.folder, Self::spelling(input.extension, role.marker))))
        })
    }

    fn misplaced_suffix(path: &Path, source: &str) -> Result<Option<Role>, Failure> {
        let names = super::declaration_names_source(path, source)?;
        let parent = path
            .parent()
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .replace('\\', "/");
        let names = names
            .iter()
            .map(String::as_str)
            .chain(path.file_stem().and_then(|name| name.to_str()));
        let misplaced = names
            .filter_map(Self::suffix_role)
            .find(|role| !role.is_placed_in(&parent));
        Ok(misplaced)
    }

    fn suffix_role(name: &str) -> Option<Role> {
        [
            "IdentityProvider",
            "ErrorHandler",
            "RequestRoute",
            "Service",
            "Store",
            "HTTPClient",
            "Delegate",
            "Converter",
            "Setup",
            "Entity",
        ]
        .iter()
        .filter_map(|marker| Role::named(marker))
        .find(|role| role.has_suffix_in(name))
    }

    fn realm(path: &Path) -> &str {
        path.components()
            .next()
            .and_then(|part| part.as_os_str().to_str())
            .unwrap_or("client")
    }

    fn failure(input: &Input<'_>, realm: &str, folder: &str, marker: &str) -> Failure {
        let spelling = Self::spelling(input.extension, marker);
        let expected = Self::expected_spelling(folder, &spelling);
        let unsupported = Realm::named(Self::environment(realm)).and_then(|checked| AnnotationRules::checked_refusal(marker, input.extension, checked)).map(|message| format!(" {message} This concrete role needs a supported implementation; it cannot be silently treated as plain helper code.")).unwrap_or_default();
        let portable = input.path.to_string_lossy().replace('\\', "/");
        let native = Self::native_package_help(input.extension);
        let at = Self::declaration_at(input);
        Failure::one(diagnostic(
            Self::CODE,
            format!(
                "'{portable}' declares concrete code in {realm}/{folder} without its required `{expected}` marker.{unsupported}"
            ),
            Some(format!(
                "Mark the intended role explicitly with `{expected}` (ADR0091). Folder names do not register code. Move plain collaborators to a supported models, shared or utils folder{native}; do not add a fake role annotation."
            )),
            source_span(&portable, at, (at + 1).min(input.source.len())),
        ))
    }

    fn native_package_help(extension: &str) -> &'static str {
        [("go", Self::NATIVE_PACKAGE_HELP)]
            .iter()
            .find(|(language, _)| *language == extension)
            .map_or("", |(_, help)| help)
    }

    fn declaration_at(input: &Input<'_>) -> usize {
        let code = input.masked_code().unwrap_or_default();
        Self::concrete_at(input.extension, &code).unwrap_or_default()
    }

    fn expected_spelling(folder: &str, spelling: &str) -> String {
        match folder {
            "controllers" => format!("{spelling}, SocketRoute, ErrorHandler or IdentityProvider"),
            _ => spelling.to_owned(),
        }
    }

    fn expected(path: &Path) -> Option<(&str, &str, &'static str)> {
        let mut parts = path.components();
        let realm = parts.next()?.as_os_str().to_str()?;
        let realm = Some(realm).filter(|realm| ["client", "server"].contains(realm))?;
        let folder = parts.next()?.as_os_str().to_str()?;
        let marker = Self::FOLDERS.iter().find(|(name, _)| *name == folder)?.1;
        Some((realm, folder, marker))
    }

    fn has_concrete_type(extension: &str, code: &str) -> bool {
        Self::concrete_at(extension, code).is_some()
    }

    fn concrete_at(extension: &str, code: &str) -> Option<usize> {
        let mut offset = 0;
        code.split_inclusive('\n').find_map(|line| {
            let start = offset + line.len() - line.trim_start().len();
            offset += line.len();
            line.split(['{', '}', ';'])
                .any(|part| Self::concrete_line(extension, part.trim()))
                .then_some(start)
        })
    }

    fn concrete_line(extension: &str, line: &str) -> bool {
        let concrete = !line.split_whitespace().any(|word| word == "abstract");
        let declaration = Declaration::skip_modifiers(line);
        let native = ["class", "struct", "object", "record", "actor"]
            .iter()
            .any(|kind| Declaration::strip_word(declaration, kind).is_some());
        let go = Self::go_struct(line);
        let declared = match extension {
            "go" => go,
            _ => native,
        };
        [concrete, declared].iter().all(|valid| *valid)
    }

    fn go_struct(line: &str) -> bool {
        let named = Declaration::go_name(line).is_some();
        let structure = line
            .split_whitespace()
            .any(|word| word.starts_with("struct"));
        [named, structure].iter().all(|valid| *valid)
    }

    // Matches the marker spellings the annotation parser accepts; a namespaced `#[tac::Service]` is refused with TY1308.
    fn spelling(extension: &str, marker: &str) -> String {
        match extension {
            "rs" | "php" | "rb" => format!("#[{marker}]"),
            "cs" => format!("[{marker}]"),
            "go" => format!("// @{marker}"),
            "py" => format!("@{}", marker.to_ascii_lowercase()),
            _ => format!("@{marker}"),
        }
    }
}
