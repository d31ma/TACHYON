use super::{InstallPlan, InstallStep, NeededTool};
use std::path::{Path, PathBuf};

pub(crate) struct TeaVmRecipe;

impl TeaVmRecipe {
    const VERSION: &'static str = "0.15.0";
    const MAVEN_CENTRAL: &'static str = "https://repo1.maven.org/maven2/org/teavm";
    const UNFINISHED_SUFFIX: &'static str = ".unfinished";
    const JARS: [(&'static str, &'static str, &'static str); 5] = [
        (
            "cli",
            "cli-0.15.0-all.jar",
            "c6a1ecb81dd90b66985ac7b0f09db55bec2076fa024428c7fe72b800b6b48cae",
        ),
        (
            "teavm-classlib",
            "teavm-classlib-0.15.0.jar",
            "7d805be4670a892a34c89d9d750a8afd65e2f8201b7b3a10343634ea19d1c410",
        ),
        (
            "teavm-jso-impl",
            "teavm-jso-impl-0.15.0.jar",
            "9a8d15205807b70d7efdfbd4c1356304954e20146e464aec3830dbc6616ef57c",
        ),
        (
            "teavm-metaprogramming-impl",
            "teavm-metaprogramming-impl-0.15.0.jar",
            "616d30aa28f7b0180a61c2289b11c77bf84c715bd79f255e1e42400edb5b6d70",
        ),
        (
            "teavm-platform",
            "teavm-platform-0.15.0.jar",
            "999d369ab15aa83c17987d048ba59560759169891de7e13a5a1c2bd93dd060a8",
        ),
    ];

    pub(crate) fn plan() -> InstallPlan {
        Self::plan_into(&crate::native::JavaWebServices::managed_teavm_folder())
    }

    pub(crate) fn plan_into(folder: &Path) -> InstallPlan {
        let unfinished = Self::unfinished(folder);
        let mut steps = vec![InstallStep::CreateFolder(unfinished.clone())];
        steps.extend(Self::JARS.iter().flat_map(|(artifact, jar, sha256)| {
            Self::jar_steps(&unfinished, artifact, jar, sha256)
        }));
        steps.push(InstallStep::MoveFolder {
            from: unfinished,
            to: folder.to_path_buf(),
        });
        let lines = vec![
            format!("TeaVM {} goes into {}.", Self::VERSION, folder.display()),
            String::from(
                "Tachyon checks each jar against the SHA-256 that it pins, and moves the folder into place only when all five match.",
            ),
            String::from("Tachyon uses this folder when TAC_TEAVM_HOME is not set."),
        ];
        InstallPlan::automatic(NeededTool::TeaVm, steps, lines)
    }

    fn jar_steps(folder: &Path, artifact: &str, jar: &str, sha256: &str) -> [InstallStep; 2] {
        let file = folder.join(jar);
        [
            InstallStep::Download {
                url: format!("{}/{artifact}/{}/{jar}", Self::MAVEN_CENTRAL, Self::VERSION),
                file: file.clone(),
            },
            InstallStep::CheckPinned {
                file,
                sha256: String::from(sha256),
            },
        ]
    }

    fn unfinished(folder: &Path) -> PathBuf {
        let mut name = folder.as_os_str().to_owned();
        name.push(Self::UNFINISHED_SUFFIX);
        PathBuf::from(name)
    }
}
