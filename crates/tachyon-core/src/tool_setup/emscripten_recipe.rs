use super::{InstallContext, InstallPlan, InstallStep, NeededTool, ToolCommand, UnpackSteps};

pub(crate) struct EmscriptenRecipe;

impl EmscriptenRecipe {
    const RUBY_SOURCES: &'static str = include_str!("../../../../runtimes/ruby-web/sources.txt");
    const EMSDK: &'static str = "emsdk";
    const EMSCRIPTEN: &'static str = "emscripten";
    const ARCHIVE: &'static str = "emsdk.tar.gz";
    const VERSION_FIELD: usize = 0;
    const HASH_FIELD: usize = 1;
    const URL_FIELD: usize = 2;

    pub(crate) fn plan(context: &InstallContext) -> InstallPlan {
        let emsdk = Self::pinned_fields(Self::EMSDK);
        let version = Self::field(&Self::pinned_fields(Self::EMSCRIPTEN), Self::VERSION_FIELD);
        let into = context.in_home(Self::EMSDK);
        let mut steps = UnpackSteps::pinned_download(
            context,
            Self::field(&emsdk, Self::URL_FIELD),
            Self::field(&emsdk, Self::HASH_FIELD),
            Self::ARCHIVE,
        );
        steps.extend(UnpackSteps::untar(&context.download(Self::ARCHIVE), &into));
        let python = Self::python_program(context);
        steps.extend(["install", "activate"].map(|action| {
            InstallStep::Run(
                ToolCommand::new(&python, &["emsdk.py", action, version]).in_folder(into.clone()),
            )
        }));
        let lines = vec![
            format!(
                "emsdk installs Emscripten {version} into {}.",
                into.display()
            ),
            String::from("Tachyon checks the emsdk download against the SHA-256 that it pins."),
            format!(
                "Load it in each new terminal: source {0}/emsdk_env.sh (on Windows: {0}\\emsdk_env.bat).",
                into.display()
            ),
        ];
        InstallPlan::automatic(NeededTool::Emscripten, steps, lines)
    }

    fn python_program(context: &InstallContext) -> String {
        if context.platform.is_windows() {
            return String::from("python");
        }
        String::from("python3")
    }

    fn field(fields: &[&'static str], index: usize) -> &'static str {
        fields.get(index).copied().unwrap_or_default()
    }

    fn pinned_fields(name: &str) -> Vec<&'static str> {
        let line = Self::RUBY_SOURCES
            .lines()
            .find(|line| line.split_whitespace().next() == Some(name))
            .unwrap_or_default();
        line.split_whitespace().skip(1).collect()
    }
}
