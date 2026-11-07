use clap::Args;
use std::path::PathBuf;
use tachyon_core::tool_setup::Consent;

#[derive(Clone, Debug, Args)]
pub struct AddOptions {
    #[arg(long, help = "Tachyon project root", default_value = ".")]
    pub project: PathBuf,
    #[arg(long, short = 'y', help = "Install every missing tool without asking")]
    pub yes: bool,
    #[arg(
        long,
        help = "Do not check or install the tools that the project needs"
    )]
    pub skip_tools: bool,
}

impl AddOptions {
    pub fn consent(&self, terminal: bool) -> Consent {
        Consent::from_options(self.yes, terminal)
    }
}
