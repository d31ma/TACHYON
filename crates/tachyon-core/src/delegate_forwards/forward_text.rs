use super::{ClassClose, DelegateBase, DelegateForwards, DispatchText, Overload};
use std::path::Path;

pub(super) struct ForwardText;

impl ForwardText {
    const BRACED: [&'static str; 3] = ["js", "ts", "php"];
    const MEMBER_STEP: &'static str = "  ";

    pub(super) fn written(
        path: &Path,
        (source, masked): (&str, &str),
        base: &DelegateBase<'_>,
        (names, overloads): (&[String], &[Overload]),
    ) -> Option<String> {
        let language = path.extension()?.to_str()?;
        let class = DelegateForwards::class_name(path);
        let (at, indentation) = Self::placed(language, masked, &class)
            .filter(|_| [names.is_empty(), overloads.is_empty()] != [true, true])?;
        let forwards = Self::forwards((language, &class), &indentation, base, names);
        let dispatchers: String = overloads
            .iter()
            .map(|overload| DispatchText::written((language, &class), &indentation, base, overload))
            .collect();
        let text = format!("{forwards}{dispatchers}");
        Some(Self::edited(source, (at, &text), overloads))
    }

    fn edited(source: &str, (at, text): (usize, &str), overloads: &[Overload]) -> String {
        let mut renamed: Vec<usize> = overloads
            .iter()
            .map(|overload| overload.renamed_at)
            .filter(|renamed_at| *renamed_at < at)
            .collect();
        renamed.sort_unstable();
        let mut edited = String::with_capacity(source.len() + text.len());
        let mut from = 0;
        for renamed_at in renamed {
            edited.push_str(&source[from..renamed_at]);
            edited.push_str(DispatchText::OWN_PREFIX);
            from = renamed_at;
        }
        edited.push_str(&source[from..at]);
        edited.push_str(text);
        edited.push_str(&source[at..]);
        edited
    }

    fn placed(language: &str, masked: &str, class: &str) -> Option<(usize, String)> {
        match language {
            "py" => ClassClose::indented(masked, class),
            "rb" => ClassClose::ruby_end(masked, class),
            "lua" => Some((masked.len(), String::new())),
            braced if Self::BRACED.contains(&braced) => ClassClose::member_slot(masked, class),
            _ => None,
        }
    }

    fn forwards(
        (language, class): (&str, &str),
        indentation: &str,
        base: &DelegateBase<'_>,
        names: &[String],
    ) -> String {
        let opening = Self::opening(language, indentation);
        let methods: String = names
            .iter()
            .map(|name| Self::forward((language, class), indentation, base, name))
            .collect();
        format!("{opening}{methods}")
    }

    fn opening(language: &str, indentation: &str) -> String {
        match language {
            "rb" => format!("\n{indentation}public\n"),
            _ => String::new(),
        }
    }

    fn forward(
        (language, class): (&str, &str),
        indentation: &str,
        base: &DelegateBase<'_>,
        name: &str,
    ) -> String {
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
            "lua" => format!(
                "\nfunction {class}:{name}(...)\n{step}return self.{field}:{name}(...)\nend\n"
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
