use super::{InstallContext, InstallPlan, NeededTool, UnpackSteps};

pub(crate) struct TinyGoRecipe;

impl TinyGoRecipe {
    const VERSION: &'static str = "0.42.0";
    const TOP: &'static str = "tinygo";
    const BIN: &'static str = "bin";
    const PINNED: [(&'static str, &'static str); 5] = [
        (
            "darwin-arm64",
            "493da3585e66c5da677a4be39402faf2041b4e45a61eb456e4547eca753a544e",
        ),
        (
            "darwin-amd64",
            "ef7b96cf59de5714a7493a696e9db141a85a99459620b00c42cb3bad14be2af2",
        ),
        (
            "linux-arm64",
            "f2f3f863e63728b87772e7306bd6030df6e6c89773a7082eb0122113cf9e453d",
        ),
        (
            "linux-amd64",
            "b87688fa2e19cee7d813cad7fd7dadb71dff3198e47125aba66ba4af5e490438",
        ),
        (
            "windows-amd64",
            "2698c3ade60d536bec71c8bb645350e264000ef59e62223588ba822b51187fc1",
        ),
    ];

    pub(crate) fn plan(context: &InstallContext) -> InstallPlan {
        let platform = context.platform.names().go;
        Self::plan_for(context, platform, Self::pinned_hash(platform))
    }

    fn pinned_hash(platform: &str) -> Option<&'static str> {
        Self::PINNED
            .iter()
            .find(|(name, _)| *name == platform)
            .map(|(_, hash)| *hash)
    }

    fn plan_for(context: &InstallContext, platform: &str, hash: Option<&str>) -> InstallPlan {
        match hash {
            Some(hash) => Self::download_plan(context, platform, hash),
            None => Self::manual_plan(),
        }
    }

    fn download_plan(context: &InstallContext, platform: &str, hash: &str) -> InstallPlan {
        let archive = format!(
            "tinygo{}.{platform}.{}",
            Self::VERSION,
            context.platform.archive_extension()
        );
        let url = format!(
            "https://github.com/tinygo-org/tinygo/releases/download/v{}/{archive}",
            Self::VERSION
        );
        let into = context.in_home(&format!("sdk/tinygo-{}", Self::VERSION));
        let mut steps = UnpackSteps::pinned_download(context, &url, hash, &archive);
        steps.extend(UnpackSteps::unpack(context, &archive, Self::TOP, &into));
        let lines = vec![
            format!("TinyGo {} goes into {}.", Self::VERSION, into.display()),
            String::from("Tachyon checks the TinyGo download against the SHA-256 that it pins."),
            UnpackSteps::path_line(context, &into.join(Self::BIN)),
        ];
        InstallPlan::automatic(NeededTool::TinyGo, steps, lines)
    }

    fn manual_plan() -> InstallPlan {
        InstallPlan::manual(
            NeededTool::TinyGo,
            vec![format!(
                "TinyGo {} has no download for this computer. Install it from https://tinygo.org/getting-started/install/, or keep the standard Go compiler: release builds use it when TinyGo is missing.",
                Self::VERSION
            )],
        )
    }
}
