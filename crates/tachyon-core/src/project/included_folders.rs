use crate::Failure;
use crate::failure::diagnostic;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::path::{Path, PathBuf};

pub(super) struct IncludedFolders {
    folders: Vec<(String, Dir)>,
}

impl IncludedFolders {
    const CODE: u16 = 1601;
    const MAX_FOLDERS: usize = 8;
    const APP_ONLY_ROOTS: [&'static str; 2] = ["client/pages", "server"];
    const HELP: &'static str = "tac.include lists folders of shared Tac code, relative to the project folder, such as [\"../kit\"]. Each folder holds client/components, client/services, client/models, client/shared and the other client folders, but no pages and no server (ADR 0144).";

    pub(super) fn open(project_root: &Path) -> Result<Self, Failure> {
        let names = crate::native::SiteSettings::include(project_root);
        Self::check_count(&names)?;
        let folders = names
            .iter()
            .map(|name| Self::opened(project_root, name))
            .collect::<Result<Vec<_>, Failure>>()?;
        Ok(Self { folders })
    }

    pub(super) fn folders(&self) -> &[(String, Dir)] {
        &self.folders
    }

    pub(super) fn repeated(name: &str, path: &Path) -> Failure {
        Self::failure(&format!(
            "The project and its included folder '{name}' both have {}. Keep one copy of the file.",
            path.display()
        ))
    }

    fn check_count(names: &[String]) -> Result<(), Failure> {
        if names.len() <= Self::MAX_FOLDERS {
            return Ok(());
        }
        Err(Self::failure("tac.include lists more than 8 folders."))
    }

    fn opened(project_root: &Path, name: &str) -> Result<(String, Dir), Failure> {
        let root = Self::folder_root(project_root, name)?;
        let folder = Self::open_folder(name, &root)?;
        Self::check_shared_only(name, &folder)?;
        Ok((String::from(name), folder))
    }

    fn folder_root(project_root: &Path, name: &str) -> Result<PathBuf, Failure> {
        let root = crate::native::SiblingFolder::resolve(project_root, "tac.include", name)
            .map_err(|message| Self::failure(&message))?;
        Self::apart_from_project(project_root, name, root)
    }

    fn apart_from_project(
        project_root: &Path,
        name: &str,
        root: PathBuf,
    ) -> Result<PathBuf, Failure> {
        let apart = [
            root.starts_with(project_root),
            project_root.starts_with(&root),
        ];
        if apart == [false, false] {
            return Ok(root);
        }
        Err(Self::failure(&format!(
            "tac.include names the folder '{name}', which is inside the project or holds it. Keep shared code in its own folder beside the project."
        )))
    }

    fn open_folder(name: &str, root: &Path) -> Result<Dir, Failure> {
        Dir::open_ambient_dir(root, ambient_authority()).map_err(|error| {
            Self::failure(&format!(
                "tac.include names the folder '{name}', which cannot be opened: {error}"
            ))
        })
    }

    fn check_shared_only(name: &str, folder: &Dir) -> Result<(), Failure> {
        let found = Self::APP_ONLY_ROOTS.iter().find(|root| folder.exists(root));
        found.map_or(Ok(()), |root| {
            Err(Self::failure(&format!(
                "The included folder '{name}' has {root}. An included folder holds shared client code only; keep pages and server code in the app."
            )))
        })
    }

    fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(String::from(Self::HELP)),
            None,
        ))
    }
}
