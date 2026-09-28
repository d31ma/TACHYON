use super::form_pattern::FormPattern;
use super::simpler_form::SimplerForm;
use std::fmt;

pub(super) struct FormSuggestion {
    path: String,
    line: usize,
    pattern: FormPattern,
    rewritten: Vec<String>,
}

impl FormSuggestion {
    const HELP_INDENT: &'static str = "        ";
    const REMOVED: &'static str = "remove the line";

    pub(super) fn new(path: &str, source: &str, form: &SimplerForm) -> Self {
        let rewritten = Self::shown_lines(source, form);
        Self {
            path: path.to_owned(),
            line: form.line(source),
            pattern: form.pattern,
            rewritten: Self::trimmed_lines(&rewritten),
        }
    }

    fn shown_lines(source: &str, form: &SimplerForm) -> String {
        if form.removes_lines(source) {
            return String::new();
        }
        form.rewritten_lines(source)
    }

    fn trimmed_lines(text: &str) -> Vec<String> {
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let margin = lines
            .iter()
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);
        lines
            .iter()
            .map(|line| line.get(margin..).unwrap_or(line).to_owned())
            .collect()
    }

    fn help(&self) -> String {
        if self.rewritten.is_empty() {
            return String::from(Self::REMOVED);
        }
        self.rewritten.join(&format!("\n{}", Self::HELP_INDENT))
    }
}

impl fmt::Display for FormSuggestion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "suggestion[TY{}]: {}",
            self.pattern.code(),
            self.pattern.message()
        )?;
        writeln!(formatter, "  --> {}:{}", self.path, self.line)?;
        writeln!(formatter, "  help: {}", self.help())
    }
}
