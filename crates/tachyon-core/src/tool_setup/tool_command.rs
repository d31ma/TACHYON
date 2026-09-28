use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolCommand {
    pub program: String,
    pub arguments: Vec<String>,
    pub folder: Option<PathBuf>,
}

impl ToolCommand {
    #[must_use]
    pub fn new(program: &str, arguments: &[&str]) -> Self {
        Self {
            program: String::from(program),
            arguments: arguments
                .iter()
                .map(|argument| String::from(*argument))
                .collect(),
            folder: None,
        }
    }

    #[must_use]
    pub fn in_folder(mut self, folder: PathBuf) -> Self {
        self.folder = Some(folder);
        self
    }

    #[must_use]
    pub fn text(&self) -> String {
        let mut parts = vec![self.program.clone()];
        parts.extend(self.arguments.iter().map(|argument| Self::quoted(argument)));
        parts.join(" ")
    }

    fn quoted(argument: &str) -> String {
        if argument.contains([' ', '&', '?', '"', '\'']) {
            return format!("\"{argument}\"");
        }
        String::from(argument)
    }
}
