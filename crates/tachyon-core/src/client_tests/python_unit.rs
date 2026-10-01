use super::{fail, stage_failure};
use crate::Failure;
use crate::external_command::ToolOutput;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub(super) struct PythonUnit;

impl PythonUnit {
    const SCRIPT: &'static str = "unit.py";
    const TIMEOUT: Duration = Duration::from_secs(30);
    const OUTPUT_LIMIT: usize = 262_144;

    pub(super) fn run(
        directory: &Path,
        portable: &str,
        lowered: &str,
        name: &str,
    ) -> Result<String, Failure> {
        Self::write(directory, portable, lowered, name)?;
        Self::execute(directory, portable)
    }

    fn write(directory: &Path, portable: &str, lowered: &str, name: &str) -> Result<(), Failure> {
        std::fs::write(directory.join(Self::SCRIPT), Self::program(lowered, name))
            .map_err(|error| stage_failure(portable, &error))
    }

    fn program(lowered: &str, name: &str) -> String {
        let unit = serde_json::to_string(name).unwrap_or_default();
        format!(
            "{lowered}\n\
             class _TacTestMain:\n\
             \x20   @staticmethod\n\
             \x20   async def report():\n\
             \x20       unit = {name}()\n\
             \x20       try:\n\
             \x20           return await unit.POST()\n\
             \x20       finally:\n\
             \x20           await unit.stopBehaviors()\n\
             \x20   @staticmethod\n\
             \x20   async def run():\n\
             \x20       try:\n\
             \x20           report = await _TacTestMain.report()\n\
             \x20       except Exception as error:\n\
             \x20           report = {{'unit': {unit}, 'failure': str(error)}}\n\
             \x20       print(_behavior_json.dumps(report, separators=(',', ':')))\n\n\
             _behavior_asyncio.run(_TacTestMain.run())\n"
        )
    }

    fn execute(directory: &Path, portable: &str) -> Result<String, Failure> {
        let mut command = Self::command(directory);
        let output =
            crate::external_command::run_sync(&mut command, Self::TIMEOUT, Self::OUTPUT_LIMIT)
                .map_err(|error| {
                    fail(
                        portable,
                        format!("Cannot run the Python test unit: {error}"),
                    )
                })?;
        Self::printed(&output, portable)
    }

    fn command(directory: &Path) -> Command {
        let runtime = std::env::var_os("YON_PYTHON_RUNTIME")
            .unwrap_or_else(|| Self::default_runtime().into());
        let mut command = Command::new(runtime);
        command
            .current_dir(directory)
            .env("TAC_TEST_UNIT", "1")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .arg(Self::SCRIPT);
        command
    }

    fn default_runtime() -> &'static str {
        if cfg!(windows) { "python" } else { "python3" }
    }

    fn printed(output: &ToolOutput, portable: &str) -> Result<String, Failure> {
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }
        let summary = String::from_utf8_lossy(&output.stderr)
            .lines()
            .take(6)
            .collect::<Vec<_>>()
            .join(" ");
        Err(fail(
            portable,
            format!("The Python test unit did not run: {summary}"),
        ))
    }
}

#[cfg(test)]
mod tests;
