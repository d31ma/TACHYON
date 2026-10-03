#![allow(clippy::needless_bitwise_bool)]

use std::path::Path;

pub(super) struct PathParts<'a> {
    pub(super) realm: &'a str,
    pub(super) folder: &'a str,
    pub(super) name: &'a str,
    pub(super) extension: &'a str,
}

impl<'a> PathParts<'a> {
    const REALMS: [&'static str; 2] = ["client", "server"];
    const ROLE_FOLDERS: [&'static str; 18] = [
        "services",
        "helpers",
        "controllers",
        "stores",
        "repositories",
        "remotes",
        "clients",
        "converters",
        "mappers",
        "configurations",
        "setups",
        "workers",
        "features",
        "entities",
        "models",
        "errors",
        "utils",
        "roles",
    ];
    const GENERATED_MARKS: [&'static str; 5] =
        [".d.ts", ".min.", ".generated.", "_generated.", ".g."];
    const SEPARATOR: char = '/';
    const UTILITY_FOLDER: &'static str = "utils";

    pub(super) fn of(path: &'a str) -> Option<Self> {
        let mut segments = path.split(Self::SEPARATOR);
        let realm = segments.next()?;
        let folder = Self::utility_folder(path).unwrap_or(segments.next()?);
        let file = path.rsplit(Self::SEPARATOR).next()?;
        let (name, extension) = file.split_once('.')?;
        let parts = Self {
            realm,
            folder,
            name,
            extension,
        };
        parts.in_scope(file).then_some(parts)
    }

    fn utility_folder(path: &str) -> Option<&'static str> {
        path.strip_prefix(crate::annotation_layout::UtilityLayout::CLIENT_ROOT)?
            .strip_prefix(Self::SEPARATOR)?;
        Some(Self::UTILITY_FOLDER)
    }

    fn in_scope(&self, file: &str) -> bool {
        let realm = Self::REALMS.contains(&self.realm);
        let folder = Self::ROLE_FOLDERS.contains(&self.folder);
        let generated = Self::GENERATED_MARKS.iter().any(|mark| file.contains(mark));
        let plain = Path::new(file)
            .extension()
            .is_some_and(|value| value == self.extension);
        realm & folder & !generated & plain
    }
}
