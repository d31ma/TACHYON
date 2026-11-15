use std::path::{Path, PathBuf};

pub(crate) struct SiblingFolder;

impl SiblingFolder {
    pub(crate) fn resolve(base: &Path, member: &str, folder: &str) -> Result<PathBuf, String> {
        let relative = Self::relative(member, folder)?;
        std::fs::canonicalize(base.join(relative)).map_err(|error| {
            format!("{member} names the folder '{folder}', which cannot be opened: {error}")
        })
    }

    fn relative<'a>(member: &str, folder: &'a str) -> Result<&'a Path, String> {
        let path = Path::new(folder);
        if path.is_relative() {
            return Ok(path);
        }
        Err(format!(
            "{member} names the folder '{folder}'. Name it relative to the project folder, such as ../kit."
        ))
    }
}
