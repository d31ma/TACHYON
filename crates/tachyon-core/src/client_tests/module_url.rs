use std::path::Path;

pub(super) struct ModuleUrl;

impl ModuleUrl {
    pub(super) fn file(path: &Path) -> std::io::Result<String> {
        Self::checked(url::Url::from_file_path(std::path::absolute(path)?))
    }

    pub(super) fn directory(path: &Path) -> std::io::Result<String> {
        Self::checked(url::Url::from_directory_path(std::path::absolute(path)?))
    }

    fn checked(url: Result<url::Url, ()>) -> std::io::Result<String> {
        url.map(String::from).map_err(|()| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "A test unit module path cannot be written as a file URL.",
            )
        })
    }
}
