use crate::add_options::AddOptions;
use crate::add_what::AddWhat;
use crate::tool_step::ToolStep;
use std::io::IsTerminal;
use tachyon_core::{
    AdditionResult, AppFeature, AppTarget, ChoiceList, Failure, ProjectAddition, RealmChoice,
};

pub struct AddCommand;

impl AddCommand {
    pub fn run(what: &AddWhat) -> Result<(), Failure> {
        let (result, options) = Self::added(what)?;
        Self::print(&result);
        let terminal = std::io::stdin().is_terminal();
        ToolStep::print_notes(&result.choices);
        ToolStep::run(
            &options.project,
            &result.choices,
            options.skip_tools,
            options.consent(terminal),
        )
    }

    fn added(what: &AddWhat) -> Result<(AdditionResult, &AddOptions), Failure> {
        match what {
            AddWhat::Language {
                realm,
                language,
                options,
            } => {
                let realm = ChoiceList::one::<RealmChoice>(realm, "realm")?;
                Ok((
                    ProjectAddition::language(&options.project, realm, language)?,
                    options,
                ))
            }
            AddWhat::Target { target, options } => {
                let target = ChoiceList::one::<AppTarget>(target, "target")?;
                Ok((ProjectAddition::target(&options.project, target)?, options))
            }
            AddWhat::Feature { feature, options } => {
                let feature = ChoiceList::one::<AppFeature>(feature, "feature")?;
                Ok((
                    ProjectAddition::feature(&options.project, feature)?,
                    options,
                ))
            }
        }
    }

    fn print(result: &AdditionResult) {
        let mut lines: Vec<String> = result
            .written
            .iter()
            .map(|path| format!("  created {path}"))
            .collect();
        lines.extend(result.notes.iter().cloned());
        println!(
            "{}",
            lines.join(
                "
"
            )
        );
    }
}
