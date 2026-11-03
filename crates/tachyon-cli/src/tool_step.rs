use std::io::IsTerminal;
use std::path::Path;
use tachyon_core::tool_setup::{
    Consent, InstallContext, InstallRecipes, NeededTool, SystemRunner, ToolCheck, ToolSetup,
};
use tachyon_core::{Failure, ProjectChoices};

pub struct ToolStep;

impl ToolStep {
    pub fn run(
        root: &Path,
        choices: &ProjectChoices,
        skip: bool,
        consent: Consent,
    ) -> Result<(), Failure> {
        Self::run_unless_skipped(root, choices, skip, consent)
    }

    pub fn install_for_project(root: &Path, answer_yes: bool) -> Result<(), Failure> {
        println!("Checking the tools that this project needs...");
        let missing = ToolCheck::missing_for_project(root)?;
        let consent = Consent::from_options(answer_yes, std::io::stdin().is_terminal());
        Self::setup(root, &missing, consent)
    }

    pub fn print_notes(choices: &ProjectChoices) {
        let notes: Vec<&str> = choices
            .tac
            .iter()
            .map(|language| language.note())
            .filter(|note| !note.is_empty())
            .collect();
        println!("{}", notes.join("\n"));
    }

    fn run_unless_skipped(
        root: &Path,
        choices: &ProjectChoices,
        skip: bool,
        consent: Consent,
    ) -> Result<(), Failure> {
        if skip {
            return Ok(());
        }
        Self::check(root, choices, consent)
    }

    fn check(root: &Path, choices: &ProjectChoices, consent: Consent) -> Result<(), Failure> {
        println!("\nChecking the tools that this app needs...");
        let missing = ToolCheck::missing(root, choices)?;
        Self::setup(root, &missing, consent)
    }

    fn setup(root: &Path, missing: &[NeededTool], consent: Consent) -> Result<(), Failure> {
        if missing.is_empty() {
            println!("Every tool that this app needs is ready.");
            return Ok(());
        }
        Self::install(root, missing, consent)
    }

    fn install(root: &Path, missing: &[NeededTool], consent: Consent) -> Result<(), Failure> {
        let context = InstallContext::for_project(root)?;
        let plans = InstallRecipes::plans(missing, &context);
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        let mut output = std::io::stdout();
        let mut runner = SystemRunner;
        ToolSetup::new(&mut input, &mut output, &mut runner, consent).run(&plans)?;
        Ok(())
    }
}
