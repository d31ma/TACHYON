use super::{ArchiveRecipe, InstallContext, InstallPlan, NeededTool};
use std::path::PathBuf;

pub(crate) struct ArchiveRecipes;

impl ArchiveRecipes {
    const BUN: &'static str = "1.3.14";
    const GO: &'static str = "1.27.1";
    const DART: &'static str = "3.13.1";
    const BIN: &'static str = "bin";

    pub(crate) fn bun(context: &InstallContext) -> InstallPlan {
        let platform = context.platform.names().bun;
        let archive = format!("bun-{platform}.zip");
        let base = format!(
            "https://github.com/oven-sh/bun/releases/download/bun-v{}",
            Self::BUN
        );
        let into = Self::bun_folder(context);
        ArchiveRecipe {
            tool: NeededTool::Bun,
            title: format!("Bun {}", Self::BUN),
            url: format!("{base}/{archive}"),
            checksums: format!("{base}/SHASUMS256.txt"),
            archive,
            top: format!("bun-{platform}"),
            bin: into.clone(),
            into,
        }
        .plan(context)
    }

    pub(crate) fn go(context: &InstallContext) -> InstallPlan {
        let archive = format!(
            "go{}.{}.{}",
            Self::GO,
            context.platform.names().go,
            context.platform.archive_extension()
        );
        let into = context.in_home(&format!("sdk/go{}", Self::GO));
        ArchiveRecipe {
            tool: NeededTool::Go,
            title: format!("Go {}", Self::GO),
            url: format!("https://go.dev/dl/{archive}"),
            checksums: format!("https://dl.google.com/go/{archive}.sha256"),
            archive,
            top: String::from("go"),
            bin: into.join(Self::BIN),
            into,
        }
        .plan(context)
    }

    pub(crate) fn dart(context: &InstallContext) -> InstallPlan {
        let archive = format!("dartsdk-{}-release.zip", context.platform.names().dart);
        let url = format!(
            "https://storage.googleapis.com/dart-archive/channels/stable/release/{}/sdk/{archive}",
            Self::DART
        );
        let into = context.in_home(&format!("sdk/dart-{}", Self::DART));
        ArchiveRecipe {
            tool: NeededTool::Dart,
            title: format!("The Dart SDK {}", Self::DART),
            checksums: format!("{url}.sha256sum"),
            url,
            archive,
            top: String::from("dart-sdk"),
            bin: into.join(Self::BIN),
            into,
        }
        .plan(context)
    }

    pub(crate) fn bun_folder(context: &InstallContext) -> PathBuf {
        context.in_home(&format!(".bun/bun-{}", Self::BUN))
    }
}
