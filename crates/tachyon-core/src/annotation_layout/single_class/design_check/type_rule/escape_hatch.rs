use super::super::class_file::ClassFile;
use super::super::design_problem::DesignProblem;
use super::super::design_rule::DesignRule;

pub(super) struct EscapeHatch;

impl EscapeHatch {
    const SCRIPT_MARKS: [&'static str; 3] = ["@ts-nocheck", "@ts-ignore", "@ts-expect-error"];
    const PYTHON_MARKS: [&'static str; 3] =
        ["type: ignore", "mypy: ignore-errors", "pyright: ignore"];
    const SCRIPT_LANGUAGES: [&'static str; 2] = ["js", "ts"];
    const PYTHON: &'static str = "py";
    const COMMENT_STARTS: [&'static str; 4] = ["//", "/*", "#", "*"];
    const NEWLINE: char = '\n';
    const HELP: &'static str = "Remove it and fix the type errors it hides. A class file keeps every type check on, because its types are only worth what the checker verifies (ADR 0079).";

    pub(super) fn problems(class: &ClassFile) -> Vec<DesignProblem> {
        Self::marks(class.extension)
            .iter()
            .flat_map(|mark| Self::found(class, mark))
            .collect()
    }

    fn marks(extension: &str) -> &'static [&'static str] {
        let script = usize::from(Self::SCRIPT_LANGUAGES.contains(&extension));
        let python = usize::from(extension == Self::PYTHON);
        let choices: [&'static [&'static str]; 3] = [&[], &Self::SCRIPT_MARKS, &Self::PYTHON_MARKS];
        choices[script + 2 * python]
    }

    fn found(class: &ClassFile, mark: &str) -> Vec<DesignProblem> {
        class
            .text
            .match_indices(mark)
            .filter(|(at, _)| Self::in_comment(class.text, *at))
            .map(|(at, _)| Self::problem(class, at, mark))
            .collect()
    }

    fn in_comment(text: &str, at: usize) -> bool {
        let line = text[..at]
            .rfind(Self::NEWLINE)
            .map_or(0, |newline| newline + 1);
        let before = &text[line..at];
        Self::COMMENT_STARTS
            .iter()
            .any(|start| before.contains(start))
    }

    fn problem(class: &ClassFile, at: usize, mark: &str) -> DesignProblem {
        DesignProblem {
            rule: DesignRule::Types,
            path: class.path.to_owned(),
            start: at,
            end: at + mark.len(),
            line: class.line(at),
            message: format!("`{mark}` in {} switches type checking off.", class.name),
            help: String::from(Self::HELP),
        }
    }
}
