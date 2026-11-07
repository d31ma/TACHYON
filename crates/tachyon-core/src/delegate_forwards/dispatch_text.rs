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
    const BODIES: [(&'static str, &'static str); 6] = [
        (
            "js",
            "\n{i}{name}(...args) {\n{i}{s}if ({condition}) {\n{i}{s}{s}return this.{own}(...args);\n{i}{s}}\n{i}{s}return this.{field}.{name}(...args);\n{i}}\n",
        ),
        (
            "ts",
            "\n{i}{name}(...args: Parameters<{service}['{name}']>): ReturnType<{service}['{name}']>;\n{i}{name}(...args: Parameters<{class}['{own}']>): ReturnType<{class}['{own}']>;\n{i}{name}(...args: any[]): any {\n{i}{s}if ({condition}) {\n{i}{s}{s}return (this.{own} as (...values: any[]) => any).apply(this, args);\n{i}{s}}\n{i}{s}return (this.{field}.{name} as (...values: any[]) => any).apply(this.{field}, args);\n{i}}\n",
        ),
        (
            "php",
            "\n{i}public function {name}(...$args) {\n{i}{s}if ({condition}) {\n{i}{s}{s}return $this->{own}(...$args);\n{i}{s}}\n{i}{s}return $this->{field}->{name}(...$args);\n{i}}\n",
        ),
        (
            "py",
            "\n\n{i}def {name}(self, *args, **kwargs):\n{i}{i}if {condition}:\n{i}{i}{i}return self.{own}(*args, **kwargs)\n{i}{i}return self.{field}.{name}(*args, **kwargs)",
        ),
        (
            "lua",
            "\nfunction {class}:{name}(...)\n{s}local count = select('#', ...)\n{s}if {condition} then\n{s}{s}return self:{own}(...)\n{s}end\n{s}return self.{field}:{name}(...)\nend\n",
        ),
        (
            "rb",
            "\n{i}def {name}(*args, **kwargs, &block)\n{i}{s}return {own}(*args, **kwargs, &block) if {condition}\n{i}{s}{field}.{name}(*args, **kwargs, &block)\n{i}end\n",
        ),
    ];

    pub(super) fn written(
        (language, class): (&str, &str),
        indentation: &str,
        base: &DelegateBase<'_>,
        overload: &Overload,
    ) -> String {
        let own = format!("{}{}", Self::OWN_PREFIX, overload.name);
        let condition = Self::condition(language, overload.own);
        let body = Self::BODIES
            .iter()
            .find(|(owner, _)| *owner == language)
            .map_or("", |(_, body)| *body);
        [
            ("{i}", indentation),
            ("{s}", Self::STEP),
            ("{name}", &overload.name),
            ("{own}", &own),
            ("{field}", &base.field),
            ("{service}", &base.service),
            ("{class}", class),
            ("{condition}", &condition),
        ]
        .into_iter()
        .fold(body.to_owned(), |text, (key, value)| {
            text.replace(key, value)
        })
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
                let joined = format!("{count} >= {required} {and} {count} <= {most}");
                let chained = format!("{required} <= {count} <= {most}");
                [joined, chained][usize::from(python)].clone()
            },
        )
    }
}
