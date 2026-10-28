use super::{ClassClose, DelegateBase, DelegateForwards, SourceLines};
use std::path::Path;

pub(super) struct ForwardText;

impl ForwardText {
    const BRACED: [&'static str; 3] = ["js", "ts", "php"];
    const MEMBER_STEP: &'static str = "  ";

    pub(super) fn written(
        path: &Path,
        (source, masked): (&str, &str),
        base: &DelegateBase<'_>,
        names: &[String],
    ) -> Option<String> {
        let language = path.extension()?.to_str()?;
        let class = DelegateForwards::class_name(path);
        let (at, text) = Self::placed(language, masked, &class)
            .map(|(at, indentation)| (at, Self::forwards(language, &indentation, base, names)))
            .filter(|_| !names.is_empty())?;
        Some(format!("{}{text}{}", &source[..at], &source[at..]))
    }

    fn placed(language: &str, masked: &str, class: &str) -> Option<(usize, String)> {
        match language {
            "py" => ClassClose::indented(masked, class),
            "rb" => ClassClose::ruby_end(masked, class),
            braced if Self::BRACED.contains(&braced) => Self::before_brace(masked, class),
            _ => None,
        }
    }

    fn before_brace(masked: &str, class: &str) -> Option<(usize, String)> {
        let at = ClassClose::braced(masked, class)?;
        let line_start = masked[..at].rfind('\n').map_or(0, |offset| offset + 1);
        let indentation = SourceLines::indentation(&masked[line_start..at]);
        Some((at, format!("{indentation}{}", Self::MEMBER_STEP)))
    }

    fn forwards(
        language: &str,
        indentation: &str,
        base: &DelegateBase<'_>,
        names: &[String],
    ) -> String {
        let opening = Self::opening(language, indentation);
        let methods: String = names
            .iter()
            .map(|name| Self::forward(language, indentation, base, name))
            .collect();
        format!("{opening}{methods}")
    }

    fn opening(language: &str, indentation: &str) -> String {
        match language {
            "rb" => format!("\n{indentation}public\n"),
            _ => String::new(),
        }
    }

    fn forward(language: &str, indentation: &str, base: &DelegateBase<'_>, name: &str) -> String {
        let field = &base.field;
        let service = &base.service;
        let step = Self::MEMBER_STEP;
        match language {
            "js" => format!(
                "\n{indentation}{name}(...args) {{\n{indentation}{step}return this.{field}.{name}(...args);\n{indentation}}}\n"
            ),
            "ts" => format!(
                "\n{indentation}{name}(...args: Parameters<{service}['{name}']>): ReturnType<{service}['{name}']> {{\n{indentation}{step}return this.{field}.{name}(...args);\n{indentation}}}\n"
            ),
            "php" => format!(
                "\n{indentation}public function {name}(...$args) {{\n{indentation}{step}return $this->{field}->{name}(...$args);\n{indentation}}}\n"
            ),
            "py" => format!(
                "\n\n{indentation}def {name}(self, *args, **kwargs):\n{indentation}{indentation}return self.{field}.{name}(*args, **kwargs)"
            ),
            _ => format!(
                "\n{indentation}def {name}(...)\n{indentation}{step}{field}.{name}(...)\n{indentation}end\n"
            ),
        }
    }
}
