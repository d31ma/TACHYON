use super::host::native_tool_failure;
use crate::Failure;
use crate::external_command::ToolOutput;

#[cfg(test)]
mod tests;

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct RustNativeLibraries {
    pub(super) arguments: Vec<String>,
}

impl RustNativeLibraries {
    const MARKER: &'static str = "native-static-libs:";
    const FRAMEWORK: &'static str = "-framework";

    pub(super) fn read(output: &ToolOutput) -> Result<Self, Failure> {
        Self::require_successful_compilation(output)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Self::parse(&format!("{stdout}\n{stderr}"))
    }

    fn require_successful_compilation(output: &ToolOutput) -> Result<(), Failure> {
        if !output.status.success() {
            return Err(native_tool_failure(
                1605,
                "The Rust compilation failed; its static library requirements cannot be used.",
            ));
        }
        Ok(())
    }

    fn parse(output: &str) -> Result<Self, Failure> {
        let text = Self::metadata(output)?;
        let arguments = text
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();
        Self::require_arguments(&arguments)?;
        Ok(Self { arguments })
    }

    fn metadata(output: &str) -> Result<&str, Failure> {
        let mut metadata = output.lines().filter_map(|line| {
            let line = line.trim().strip_prefix("note: ").unwrap_or(line.trim());
            line.strip_prefix(Self::MARKER)
        });
        let text = metadata.next().ok_or_else(Self::missing)?;
        if metadata.next().is_some() {
            return Err(Self::invalid());
        }
        Ok(text)
    }

    fn require_arguments(arguments: &[String]) -> Result<(), Failure> {
        let mut index = 0;
        while index < arguments.len() {
            index += Self::argument_width(arguments, index)?;
        }
        Ok(())
    }

    fn argument_width(arguments: &[String], index: usize) -> Result<usize, Failure> {
        let argument = &arguments[index];
        if argument == Self::FRAMEWORK {
            let name = arguments.get(index + 1).ok_or_else(Self::invalid)?;
            Self::require_name(name)?;
            return Ok(2);
        }
        let name = argument.strip_prefix("-l").ok_or_else(Self::invalid)?;
        Self::require_name(name)?;
        Ok(1)
    }

    fn require_name(name: &str) -> Result<(), Failure> {
        Self::require_nonempty_name(name)?;
        Self::require_name_without_flag_prefix(name)?;
        name.chars()
            .all(Self::valid_character)
            .then_some(())
            .ok_or_else(Self::invalid)
    }

    fn require_nonempty_name(name: &str) -> Result<(), Failure> {
        if name.is_empty() {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn require_name_without_flag_prefix(name: &str) -> Result<(), Failure> {
        if name.starts_with('-') {
            return Err(Self::invalid());
        }
        Ok(())
    }

    fn valid_character(character: char) -> bool {
        match character {
            '_' | '-' | '.' | '+' => true,
            _ => character.is_ascii_alphanumeric(),
        }
    }

    fn missing() -> Failure {
        native_tool_failure(
            1605,
            "Rust did not report the native libraries required by its static companion. \
             The macOS host cannot safely link this archive.",
        )
    }

    fn invalid() -> Failure {
        native_tool_failure(
            1605,
            "Rust reported ambiguous or unsupported native static library arguments. \
             Expected -l<library> or -framework <name> arguments.",
        )
    }
}
