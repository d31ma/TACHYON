use super::ToolCommand;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstallStep {
    Download { url: String, file: PathBuf },
    CheckPublished { file: PathBuf, list: PathBuf },
    CheckPinned { file: PathBuf, sha256: String },
    CreateFolder(PathBuf),
    MoveFolder { from: PathBuf, to: PathBuf },
    Run(ToolCommand),
}

impl InstallStep {
    #[must_use]
    pub fn text(&self) -> String {
        self.described()
    }

    fn described(&self) -> String {
        match self {
            Self::Download { url, file } => format!("download {url} to {}", file.display()),
            Self::CheckPublished { file, list } => format!(
                "check the SHA-256 of {} against the vendor's published {}",
                file.display(),
                list.display()
            ),
            Self::CheckPinned { file, sha256 } => {
                format!("check that the SHA-256 of {} is {sha256}", file.display())
            }
            Self::CreateFolder(folder) => format!("create the folder {}", folder.display()),
            Self::MoveFolder { from, to } => {
                format!("move {} to {}", from.display(), to.display())
            }
            Self::Run(command) => command.text(),
        }
    }
}
