use super::{
    InstallContext, InstallPlan, InstallStep, NeededTool, PlanKind, ToolCommand, UnpackSteps,
};

type Candidate = (NeededTool, &'static str, &'static str, &'static str);

pub(crate) struct SdkmanRecipes;

impl SdkmanRecipes {
    const SDKMAN_INIT: &'static str = ".sdkman/bin/sdkman-init.sh";
    const SDKMAN_SCRIPT: &'static str = "sdkman-install.sh";
    const BASH: &'static str = "bash";
    const JAVA: Candidate = (
        NeededTool::Java,
        "java",
        "21.0.12+1.1-tem",
        "Download the Temurin 21 JDK installer from https://adoptium.net/temurin/releases/?version=21 and run it for your user. Then open a new terminal.",
    );
    const GROOVY: Candidate = (
        NeededTool::Groovy,
        "groovy",
        "5.1.3",
        "Download apache-groovy-binary-5.1.3.zip from https://groovy.apache.org/download.html (the page links its SHA-256), unzip it into your home folder, and add its bin folder to your Path.",
    );
    const KOTLIN: Candidate = (
        NeededTool::Kotlin,
        "kotlin",
        "2.1.10",
        "Download kotlin-compiler-2.1.10.zip from https://github.com/JetBrains/kotlin/releases/tag/v2.1.10 (the page lists its SHA-256), unzip it into your home folder, and add its bin folder to your Path.",
    );
    const GRADLE: Candidate = (
        NeededTool::Gradle,
        "gradle",
        "8.14.5",
        "Download gradle-8.14.5-bin.zip from https://gradle.org/releases/ (the page links its SHA-256), unzip it into your home folder, and add its bin folder to your Path.",
    );

    pub(crate) fn java(context: &InstallContext) -> InstallPlan {
        Self::candidate(context, Self::JAVA)
    }

    pub(crate) fn groovy(context: &InstallContext) -> InstallPlan {
        Self::candidate(context, Self::GROOVY)
    }

    pub(crate) fn kotlin(context: &InstallContext) -> InstallPlan {
        Self::candidate(context, Self::KOTLIN)
    }

    pub(crate) fn gradle(context: &InstallContext) -> InstallPlan {
        Self::candidate(context, Self::GRADLE)
    }

    fn candidate(context: &InstallContext, candidate: Candidate) -> InstallPlan {
        if context.platform.is_windows() {
            return InstallPlan::advice(
                candidate.0,
                PlanKind::Manual,
                vec![String::from(candidate.3)],
            );
        }
        Self::with_sdkman(context, candidate)
    }

    fn with_sdkman(context: &InstallContext, candidate: Candidate) -> InstallPlan {
        let (tool, name, version, _) = candidate;
        let mut steps = Self::sdkman_steps(context);
        let script = format!(
            "export sdkman_auto_answer=true; source \"$HOME/{}\"; sdk install {name} {version}",
            Self::SDKMAN_INIT
        );
        steps.push(InstallStep::Run(ToolCommand::new(
            Self::BASH,
            &["-c", &script],
        )));
        let lines = vec![
            format!(
                "SDKMAN installs {name} {version} into {}.",
                context.in_home(".sdkman/candidates").display()
            ),
            String::from(
                "SDKMAN publishes no checksum for its install script. SDKMAN checks each tool that it downloads.",
            ),
            format!(
                "Open a new terminal afterwards, or run: source \"$HOME/{}\"",
                Self::SDKMAN_INIT
            ),
        ];
        InstallPlan::automatic(tool, steps, lines)
    }

    fn sdkman_steps(context: &InstallContext) -> Vec<InstallStep> {
        if context.in_home(Self::SDKMAN_INIT).is_file() {
            return Vec::new();
        }
        let script = context.download(Self::SDKMAN_SCRIPT);
        let mut steps = UnpackSteps::unverified_download(
            context,
            "https://get.sdkman.io?rcupdate=false",
            Self::SDKMAN_SCRIPT,
        );
        steps.push(InstallStep::Run(ToolCommand::new(
            "bash",
            &[&script.to_string_lossy()],
        )));
        steps
    }
}
