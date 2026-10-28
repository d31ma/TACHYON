use super::{ArgumentRange, DelegateBase, Overload};

pub(super) struct DispatchText;

impl DispatchText {
    pub(super) const OWN_PREFIX: &'static str = "_own_";
    const STEP: &'static str = "  ";
    const COUNTS: [(&'static str, &'static str); 6] = [
        ("js", "args.length"),
        ("ts", "args.length"),
        ("php", "count($args)"),
        ("py", "len(args) + len(kwargs)"),
        ("rb", "args.length + kwargs.length"),
        ("lua", "count"),
    ];
    const AND_WORDS: [(&'static str, &'static str); 2] = [("lua", "and"), ("rb", "&&")];

    pub(super) fn written(
        (language, class): (&str, &str),
        indentation: &str,
        base: &DelegateBase<'_>,
        overload: &Overload,
    ) -> String {
        let step = Self::STEP;
        let name = &overload.name;
        let own = format!("{}{name}", Self::OWN_PREFIX);
        let field = &base.field;
        let service = &base.service;
        let condition = Self::condition(language, overload.own);
        match language {
            "js" => format!(
                "\n{indentation}{name}(...args) {{\n{indentation}{step}if ({condition}) {{\n{indentation}{step}{step}return this.{own}(...args);\n{indentation}{step}}}\n{indentation}{step}return this.{field}.{name}(...args);\n{indentation}}}\n"
            ),
            "ts" => format!(
                "\n{indentation}{name}(...args: Parameters<{service}['{name}']>): ReturnType<{service}['{name}']>;\n{indentation}{name}(...args: Parameters<{class}['{own}']>): ReturnType<{class}['{own}']>;\n{indentation}{name}(...args: any[]): any {{\n{indentation}{step}if ({condition}) {{\n{indentation}{step}{step}return (this.{own} as (...values: any[]) => any).apply(this, args);\n{indentation}{step}}}\n{indentation}{step}return (this.{field}.{name} as (...values: any[]) => any).apply(this.{field}, args);\n{indentation}}}\n"
            ),
            "php" => format!(
                "\n{indentation}public function {name}(...$args) {{\n{indentation}{step}if ({condition}) {{\n{indentation}{step}{step}return $this->{own}(...$args);\n{indentation}{step}}}\n{indentation}{step}return $this->{field}->{name}(...$args);\n{indentation}}}\n"
            ),
            "py" => format!(
                "\n\n{indentation}def {name}(self, *args, **kwargs):\n{indentation}{indentation}if {condition}:\n{indentation}{indentation}{indentation}return self.{own}(*args, **kwargs)\n{indentation}{indentation}return self.{field}.{name}(*args, **kwargs)"
            ),
            "lua" => format!(
                "\nfunction {class}:{name}(...)\n{step}local count = select('#', ...)\n{step}if {condition} then\n{step}{step}return self:{own}(...)\n{step}end\n{step}return self.{field}:{name}(...)\nend\n"
            ),
            _ => format!(
                "\n{indentation}def {name}(*args, **kwargs, &block)\n{indentation}{step}return {own}(*args, **kwargs, &block) if {condition}\n{indentation}{step}{field}.{name}(*args, **kwargs, &block)\n{indentation}end\n"
            ),
        }
    }

    fn condition(language: &str, range: ArgumentRange) -> String {
        let count = Self::COUNTS
            .iter()
            .find(|(owner, _)| *owner == language)
            .map_or("", |(_, count)| *count);
        let required = range.required;
        let and = Self::AND_WORDS
            .iter()
            .find(|(owner, _)| *owner == language)
            .map_or("&&", |(_, word)| *word);
        let python = language == "py";
        range.most.map_or_else(
            || format!("{count} >= {required}"),
            |most| {
                python
                    .then(|| format!("{required} <= {count} <= {most}"))
                    .unwrap_or_else(|| format!("{count} >= {required} {and} {count} <= {most}"))
            },
        )
    }
}
