use crate::init_choices::InitChoices;
use crate::tool_step::ToolStep;
use std::io::IsTerminal;
use std::path::Path;
use tachyon_core::{ChoicePrompt, ComposedTypes, Failure, ProjectChoices, Scaffold};

pub struct InitCommand;

impl InitCommand {
    const NAME_VARIABLE: &'static str = "TAC_APP_NAME";

    pub fn run(target: &Path, name: Option<&str>, flags: &InitChoices) -> Result<(), Failure> {
        let environment_name = std::env::var(Self::NAME_VARIABLE).ok();
        let name = name.or(environment_name.as_deref());
        let terminal = std::io::stdin().is_terminal();
        let choices = Self::choices(flags, terminal)?;
        let result = Scaffold::create_with(target, name, &choices)?;
        ComposedTypes::write(result.root())?;
        println!(
            "\nInitialized '{}' in {}",
            result.app_name(),
            result.root().display()
        );
        ToolStep::print_notes(&choices);
        ToolStep::run(
            result.root(),
            &choices,
            flags.skip_tools,
            flags.consent(terminal),
        )?;
        println!(
            "\nNext:\n  cd {}\n  ty test --coverage\n  ty preview",
            result.root().display()
        );
        Ok(())
    }

    fn choices(flags: &InitChoices, terminal: bool) -> Result<ProjectChoices, Failure> {
        if flags.should_ask(terminal) {
            return Self::prompted();
        }
        ProjectChoices::from_answers(&flags.answers())
    }

    fn prompted() -> Result<ProjectChoices, Failure> {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        let mut output = std::io::stdout();
        ChoicePrompt::new(&mut input, &mut output).ask()
    }
}
