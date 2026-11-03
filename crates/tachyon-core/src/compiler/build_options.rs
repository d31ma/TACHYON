use super::BuildProfile;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildOptions {
    pub output_directory: PathBuf,
    pub incremental: bool,
    pub profile: BuildProfile,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            output_directory: PathBuf::from("dist"),
            incremental: true,
            profile: BuildProfile::Release,
        }
    }
}
