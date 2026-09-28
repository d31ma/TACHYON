use super::{
    Consent, InstallPlan, InstallStep, NeededTool, PlanKind, Sha256Check, ToolCommand, ToolRunner,
};
use crate::Failure;
use crate::failure::diagnostic;
use std::io::{BufRead, Write};
use std::path::Path;

pub struct ToolSetup<'a> {
    input: &'a mut dyn BufRead,
    output: &'a mut dyn Write,
    runner: &'a mut dyn ToolRunner,
    consent: Consent,
}

impl<'a> ToolSetup<'a> {
    const CODE: u16 = 1409;
    const YES: [&'static str; 2] = ["y", "yes"];
    const ADMIN: &'static str =
        "It needs administrator rights, so Tachyon does not run it. Run these commands yourself:";
    const MANUAL: &'static str = "Tachyon cannot install it for you. Follow these steps:";
    const SKIPPED: &'static str = "Skipped. You can run the commands above yourself later.\n";
    const NOT_ASKED: &'static str = "Run the command again with --yes to let Tachyon install it, or run the commands yourself.\n";

    pub fn new(
        input: &'a mut dyn BufRead,
        output: &'a mut dyn Write,
        runner: &'a mut dyn ToolRunner,
        consent: Consent,
    ) -> Self {
        Self {
            input,
            output,
            runner,
            consent,
        }
    }

    pub fn run(&mut self, plans: &[InstallPlan]) -> Result<Vec<NeededTool>, Failure> {
        self.say(&format!(
            "\nThis app needs {} tool(s) that this computer does not have yet.\n",
            plans.len()
        ))?;
        self.handle_all(plans)
    }

    fn handle_all(&mut self, plans: &[InstallPlan]) -> Result<Vec<NeededTool>, Failure> {
        let mut still_missing = Vec::new();
        for plan in plans {
            still_missing.extend(self.handle(plan)?);
        }
        Ok(still_missing)
    }

    fn handle(&mut self, plan: &InstallPlan) -> Result<Option<NeededTool>, Failure> {
        match plan.kind {
            PlanKind::Automatic => self.offer(plan),
            PlanKind::NeedsAdmin => self.explain(plan, Self::ADMIN),
            PlanKind::Manual => self.explain(plan, Self::MANUAL),
        }
    }

    fn offer(&mut self, plan: &InstallPlan) -> Result<Option<NeededTool>, Failure> {
        let commands: Vec<String> = plan.steps.iter().map(InstallStep::text).collect();

        self.say(&format!(
            "\n{} is missing. Tachyon can install it with these steps:\n{}{}",
            Self::capitalized(plan.tool.label()),
            Self::indented(&commands),
            Self::indented(&plan.lines)
        ))?;
        let accepted = self.accepted(plan.tool)?;
        self.install_if(accepted, plan)
    }

    fn explain(&mut self, plan: &InstallPlan, reason: &str) -> Result<Option<NeededTool>, Failure> {
        self.say(&format!(
            "\n{} is missing. {reason}\n{}",
            Self::capitalized(plan.tool.label()),
            Self::indented(&plan.lines)
        ))?;
        Ok(Some(plan.tool))
    }

    fn accepted(&mut self, tool: NeededTool) -> Result<bool, Failure> {
        match self.consent {
            Consent::Always => Ok(true),
            Consent::Ask => self.asked(tool),
            Consent::Never => self.said(Self::NOT_ASKED, false),
        }
    }

    fn asked(&mut self, tool: NeededTool) -> Result<bool, Failure> {
        self.say(&format!(
            "Install {} now? Type y for yes or n for no [n]: ",
            tool.label()
        ))?;
        let mut reply = String::new();
        self.input
            .read_line(&mut reply)
            .map_err(|error| Self::failure(&format!("Cannot read the answer: {error}")))?;
        Ok(Self::YES.contains(&reply.trim().to_ascii_lowercase().as_str()))
    }

    fn install_if(
        &mut self,
        accepted: bool,
        plan: &InstallPlan,
    ) -> Result<Option<NeededTool>, Failure> {
        if !accepted {
            self.say(Self::SKIPPED)?;
            return Ok(Some(plan.tool));
        }
        self.install(plan)?;
        Ok(None)
    }

    fn install(&mut self, plan: &InstallPlan) -> Result<(), Failure> {
        plan.steps.iter().try_for_each(|step| self.execute(step))?;
        self.say(&format!(
            "{} is installed.\n",
            Self::capitalized(plan.tool.label())
        ))
    }

    fn execute(&mut self, step: &InstallStep) -> Result<(), Failure> {
        self.say(&format!("> {}\n", step.text()))?;
        match step {
            InstallStep::Download { url, file } => self.runner.run(&Self::curl(url, file)),
            InstallStep::CheckPublished { file, list } => Sha256Check::published(file, list),
            InstallStep::CheckPinned { file, sha256 } => Sha256Check::pinned(file, sha256),
            InstallStep::CreateFolder(folder) => Self::create_folder(folder),
            InstallStep::MoveFolder { from, to } => Self::move_folder(from, to),
            InstallStep::Run(command) => self.runner.run(command),
        }
    }

    fn curl(url: &str, file: &Path) -> ToolCommand {
        let file_text = file.to_string_lossy();
        ToolCommand::new(
            "curl",
            &[
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--output",
                &file_text,
                url,
            ],
        )
    }

    fn create_folder(folder: &Path) -> Result<(), Failure> {
        std::fs::create_dir_all(folder)
            .map_err(|error| Self::failure(&format!("Cannot create {}: {error}", folder.display())))
    }

    fn move_folder(from: &Path, to: &Path) -> Result<(), Failure> {
        std::fs::rename(from, to).map_err(|error| {
            Self::failure(&format!(
                "Cannot move {} to {}: {error}",
                from.display(),
                to.display()
            ))
        })
    }

    fn said(&mut self, text: &str, answer: bool) -> Result<bool, Failure> {
        self.say(text)?;
        Ok(answer)
    }

    fn say(&mut self, text: &str) -> Result<(), Failure> {
        self.output
            .write_all(text.as_bytes())
            .and_then(|()| self.output.flush())
            .map_err(|error| Self::failure(&format!("Cannot write to the terminal: {error}")))
    }

    fn indented(lines: &[String]) -> String {
        let indented: Vec<String> = lines.iter().map(|line| format!("  {line}\n")).collect();
        indented.concat()
    }

    fn capitalized(text: &str) -> String {
        let mut characters = text.chars();
        let first: String = characters
            .next()
            .map(|character| character.to_uppercase().collect())
            .unwrap_or_default();
        format!("{first}{}", characters.as_str())
    }

    fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(Self::CODE, message, None, None))
    }
}
