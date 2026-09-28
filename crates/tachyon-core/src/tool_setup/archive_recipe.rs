use super::{InstallContext, InstallPlan, NeededTool, UnpackSteps};
use std::path::PathBuf;

pub(crate) struct ArchiveRecipe {
    pub(crate) tool: NeededTool,
    pub(crate) title: String,
    pub(crate) archive: String,
    pub(crate) url: String,
    pub(crate) checksums: String,
    pub(crate) top: String,
    pub(crate) into: PathBuf,
    pub(crate) bin: PathBuf,
}

impl ArchiveRecipe {
    pub(crate) fn plan(self, context: &InstallContext) -> InstallPlan {
        let mut steps =
            UnpackSteps::verified_download(context, &self.url, &self.checksums, &self.archive);
        steps.extend(UnpackSteps::unpack(
            context,
            &self.archive,
            &self.top,
            &self.into,
        ));
        let lines = vec![
            format!("{} goes into {}.", self.title, self.into.display()),
            UnpackSteps::path_line(context, &self.bin),
        ];
        InstallPlan::automatic(self.tool, steps, lines)
    }
}
