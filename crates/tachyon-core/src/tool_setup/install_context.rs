use super::HostPlatform;
use crate::Failure;
use crate::failure::diagnostic;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallContext {
    pub platform: HostPlatform,
    pub home: PathBuf,
    pub project: PathBuf,
}

impl InstallContext {
    const HOME_VARIABLES: [&'static str; 2] = ["HOME", "USERPROFILE"];
    const DOWNLOADS: &'static str = ".tachyon/downloads";

    pub fn for_project(project: &Path) -> Result<Self, Failure> {
        let home = Self::home_folder().ok_or_else(Self::no_home)?;
        Ok(Self {
            platform: HostPlatform::current(),
            home,
            project: project.to_path_buf(),
        })
    }

    #[must_use]
    pub fn in_home(&self, relative: &str) -> PathBuf {
        self.home.join(relative)
    }

    #[must_use]
    pub fn download(&self, name: &str) -> PathBuf {
        self.in_home(Self::DOWNLOADS).join(name)
    }

    #[must_use]
    pub fn downloads(&self) -> PathBuf {
        self.in_home(Self::DOWNLOADS)
    }

    #[must_use]
    pub fn program(&self, name: &str, installed: &Path) -> String {
        Self::on_path_or(name, installed)
    }

    fn on_path_or(name: &str, installed: &Path) -> String {
        if crate::doctor::CommandProbe::output(name, &["--version"]).is_some() {
            return String::from(name);
        }
        installed.to_string_lossy().into_owned()
    }

    fn home_folder() -> Option<PathBuf> {
        Self::HOME_VARIABLES
            .iter()
            .find_map(std::env::var_os)
            .map(PathBuf::from)
    }

    fn no_home() -> Failure {
        Failure::one(diagnostic(
            1409,
            "Tachyon cannot find your home folder, so it cannot install tools there.",
            Some(String::from(
                "Set HOME (or USERPROFILE on Windows) and try again.",
            )),
            None,
        ))
    }
}
