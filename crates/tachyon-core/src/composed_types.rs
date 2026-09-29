mod composition_check;
mod declaration;
mod exported_class;
mod model_declaration;
mod realm_types;
mod test_unit_declaration;

use crate::Failure;
use crate::failure::diagnostic;
use cap_fs_ext::DirExt as _;
use cap_std::fs::Dir;
use realm_types::RealmTypes;
use std::io::ErrorKind;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub struct ComposedTypes;

impl ComposedTypes {
    const REALMS: [&'static str; 2] = ["client", "server"];
    const OUTPUT_FOLDERS: [&'static str; 2] = [".tachyon", "types"];
    const FILE_NAME: &'static str = "composed.d.ts";

    pub fn write(project_root: &Path) -> Result<(), Failure> {
        let project = Dir::open_ambient_dir(project_root, cap_std::ambient_authority())
            .map_err(|error| Self::write_failure(&error))?;
        Self::REALMS
            .iter()
            .try_for_each(|realm| Self::write_realm_types(&project, realm))
    }

    fn write_realm_types(project: &Dir, realm: &str) -> Result<(), Failure> {
        let text = RealmTypes::render(project, realm);
        Self::write_realm(project, realm, &text).map_err(|error| Self::write_failure(&error))
    }

    fn write_realm(project: &Dir, realm: &str, text: &str) -> std::io::Result<()> {
        let directory = Self::output_directory(project, realm)?;
        Self::write_when_changed(&directory, text)
    }

    fn output_directory(project: &Dir, realm: &str) -> std::io::Result<Dir> {
        let mut directory = project.try_clone()?;
        for name in Self::OUTPUT_FOLDERS {
            directory = Self::open_or_create(&directory, name)?;
        }
        Self::open_or_create(&directory, realm)
    }

    fn write_when_changed(directory: &Dir, text: &str) -> std::io::Result<()> {
        let current = directory.read_to_string(Self::FILE_NAME);
        if current.is_ok_and(|current| current == text) {
            return Ok(());
        }
        directory.write(Self::FILE_NAME, text)
    }

    fn open_or_create(parent: &Dir, name: &str) -> std::io::Result<Dir> {
        if let Err(error) = parent.create_dir(name)
            && error.kind() != ErrorKind::AlreadyExists
        {
            return Err(error);
        }
        parent.open_dir_nofollow(name)
    }

    fn write_failure(error: &std::io::Error) -> Failure {
        Failure::one(diagnostic(
            1410,
            format!("Cannot write the composed TypeScript types in .tachyon/types: {error}"),
            Some(String::from(
                "Keep .tachyon/types a regular directory inside the project, not a symlink.",
            )),
            None,
        ))
    }
}
