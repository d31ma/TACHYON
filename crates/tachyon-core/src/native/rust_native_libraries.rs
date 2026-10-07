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
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Self::parse(&format!("{stdout}\n{stderr}"))
    }

    fn parse(output: &str) -> Result<Self, Failure> {
        let mut metadata = output.lines().filter_map(|line| {
            let line = line.trim().strip_prefix("note: ").unwrap_or(line.trim());
            line.strip_prefix(Self::MARKER)
        });
        let text = metadata.next().ok_or_else(Self::missing)?;
        if metadata.next().is_some() {
            return Err(Self::invalid());
        }
        let arguments = text
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();
        let mut index = 0;
        while index < arguments.len() {
            let argument = &arguments[index];
            if argument == Self::FRAMEWORK {
                index += 1;
                let name = arguments.get(index).ok_or_else(Self::invalid)?;
                Self::valid_name(name)
                    .then_some(())
                    .ok_or_else(Self::invalid)?;
            } else {
                let name = argument.strip_prefix("-l").ok_or_else(Self::invalid)?;
                Self::valid_name(name)
                    .then_some(())
                    .ok_or_else(Self::invalid)?;
            }
            index += 1;
        }
        Ok(Self { arguments })
    }

    fn valid_name(name: &str) -> bool {
        !name.is_empty()
            && !name.starts_with('-')
            && name.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '+')
            })
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
