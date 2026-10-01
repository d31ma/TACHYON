use super::{Declaration, Failure, Input, LuaView, Path, Role, primary_role};
use crate::annotation_rules::AnnotationRules;
use crate::failure::{diagnostic, source_span};

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

pub(crate) struct FolderRoleCheck;

impl FolderRoleCheck {
    pub(crate) const CODE: u16 = 1363;
    const FOLDERS: [(&'static str, &'static str); 9] = [
        ("services", "Service"),
        ("stores", "Store"),
        ("remotes", "HTTPClient"),
        ("clients", "HTTPClient"),
        ("setups", "Setup"),
        ("entities", "Entity"),
        ("helpers", "Helper"),
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
            Err(input.failure(0, "A role class suffix must match its realm role folder", format!("Move the class to {}/{} and mark it with {} (ADR0091). Nested organization is allowed; helpers cannot hide a Service class.", Self::realm(path), role.folder, Self::spelling(input.extension, role.marker, Self::environment(Self::realm(path))))))
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
            "Helper",
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
        let spelling = Self::spelling(input.extension, marker, Self::environment(realm));
        let expected = Self::expected_spelling(folder, &spelling);
        let unsupported = AnnotationRules::unavailable(marker, input.extension, Self::environment(realm)).map(|message| format!(" {message} This concrete role needs a supported implementation; it cannot be silently treated as plain helper code.")).unwrap_or_default();
        let portable = input.path.to_string_lossy().replace('\\', "/");
        Failure::one(diagnostic(
            Self::CODE,
            format!(
                "'{portable}' declares concrete code in {realm}/{folder} without its required {expected} marker.{unsupported}"
            ),
            Some(format!(
                "Mark the intended role explicitly with `{expected}` (ADR0091). Folder names do not register code. Move plain collaborators to a supported models, shared or utils folder; do not add a fake role annotation."
            )),
            source_span(&portable, 0, 0),
        ))
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
        code.lines()
            .flat_map(|line| line.split(['{', '}', ';']))
            .any(|line| Self::concrete_line(extension, line.trim()))
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

    fn spelling(extension: &str, marker: &str, environment: &str) -> String {
        match extension {
            "rs" => format!("#[{environment}::{marker}]"),
            "php" | "rb" => format!("#[{marker}]"),
            "cs" => format!("[{marker}]"),
            "go" => format!("// @{environment}::{marker}"),
            "py" => format!("@{}", marker.to_ascii_lowercase()),
            _ => format!("@{marker}"),
        }
    }
}
