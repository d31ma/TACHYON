use crate::compiler::TypeScriptCompiler;
use crate::program_probe::ProgramProbe;
use std::path::{Path, PathBuf};

pub(crate) struct RequiredToolchain;

impl RequiredToolchain {
    const REQUIREMENT: &'static str = "TAC_TEST_UNITS_REQUIRE_TOOLCHAINS";
    const CONTINUOUS_INTEGRATION: &'static str = "CI";

    pub(crate) fn answers(program: &str, flag: &str) -> bool {
        let found = ProgramProbe::answers(program, flag);
        Self::checked(program, found)
    }

    pub(crate) fn typescript() -> Option<PathBuf> {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let program = TypeScriptCompiler::program(&workspace);
        let found = ProgramProbe::answers(&program.to_string_lossy(), "--version");
        Self::checked("tsc", found).then_some(program)
    }

    pub(crate) fn checked(program: &str, found: bool) -> bool {
        let required = [Self::CONTINUOUS_INTEGRATION, Self::REQUIREMENT]
            .iter()
            .any(|variable| std::env::var_os(variable).is_some());
        assert!(
            found || !required,
            "{program} is unavailable, but CI or {} is set, so this test must run instead of skipping",
            Self::REQUIREMENT
        );
        if !found {
            eprintln!("skipping: {program} is unavailable");
        }
        found
    }
}
