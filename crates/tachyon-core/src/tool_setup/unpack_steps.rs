use super::{InstallContext, InstallStep, ToolCommand};
use std::path::Path;

pub(crate) struct UnpackSteps;

impl UnpackSteps {
    const LIST_SUFFIX: &'static str = ".sha256-list";
    const ZIP: &'static str = ".zip";

    pub(crate) fn verified_download(
        context: &InstallContext,
        url: &str,
        list_url: &str,
        name: &str,
    ) -> Vec<InstallStep> {
        let file = context.download(name);
        let list = context.download(&format!("{name}{}", Self::LIST_SUFFIX));
        vec![
            InstallStep::CreateFolder(context.downloads()),
            InstallStep::Download {
                url: String::from(url),
                file: file.clone(),
            },
            InstallStep::Download {
                url: String::from(list_url),
                file: list.clone(),
            },
            InstallStep::CheckPublished { file, list },
        ]
    }

    pub(crate) fn pinned_download(
        context: &InstallContext,
        url: &str,
        sha256: &str,
        name: &str,
    ) -> Vec<InstallStep> {
        let file = context.download(name);
        vec![
            InstallStep::CreateFolder(context.downloads()),
            InstallStep::Download {
                url: String::from(url),
                file: file.clone(),
            },
            InstallStep::CheckPinned {
                file,
                sha256: String::from(sha256),
            },
        ]
    }

    pub(crate) fn unverified_download(
        context: &InstallContext,
        url: &str,
        name: &str,
    ) -> Vec<InstallStep> {
        vec![
            InstallStep::CreateFolder(context.downloads()),
            InstallStep::Download {
                url: String::from(url),
                file: context.download(name),
            },
        ]
    }

    pub(crate) fn unpack(
        context: &InstallContext,
        name: &str,
        top: &str,
        into: &Path,
    ) -> Vec<InstallStep> {
        Self::unpack_by_kind(context, name, top, into)
    }

    pub(crate) fn path_line(context: &InstallContext, folder: &Path) -> String {
        Self::path_line_for(context, folder)
    }

    fn unpack_by_kind(
        context: &InstallContext,
        name: &str,
        top: &str,
        into: &Path,
    ) -> Vec<InstallStep> {
        let archive = context.download(name);
        if name.ends_with(Self::ZIP) {
            return context.platform.unpack_zip(&archive, top, into);
        }
        Self::untar(&archive, into)
    }

    pub(crate) fn untar(archive: &Path, into: &Path) -> Vec<InstallStep> {
        let archive_text = archive.to_string_lossy();
        let into_text = into.to_string_lossy();
        vec![
            InstallStep::CreateFolder(into.to_path_buf()),
            InstallStep::Run(ToolCommand::new(
                "tar",
                &[
                    "-xzf",
                    &archive_text,
                    "-C",
                    &into_text,
                    "--strip-components=1",
                ],
            )),
        ]
    }

    fn path_line_for(context: &InstallContext, folder: &Path) -> String {
        let folder_text = folder.display();
        if context.platform.is_windows() {
            return format!("Add {folder_text} to your Path: setx PATH \"%PATH%;{folder_text}\"");
        }
        format!(
            "Add {folder_text} to your PATH, for example in ~/.profile: export PATH=\"{folder_text}:$PATH\""
        )
    }
}
