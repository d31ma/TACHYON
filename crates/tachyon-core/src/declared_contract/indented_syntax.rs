#[derive(Clone, Copy)]
pub(crate) struct IndentedSyntax {
    pub(crate) ruby: bool,
    pub(crate) marker_prefix: &'static str,
    pub(crate) methods: &'static [&'static str],
    pub(crate) inline_bodies: bool,
    pub(crate) closing: &'static str,
}

impl IndentedSyntax {
    const SYNTAXES: [Self; 2] = [
        Self {
            ruby: false,
            marker_prefix: "@",
            methods: &["def ", "async def "],
            inline_bodies: true,
            closing: "\n",
        },
        Self {
            ruby: true,
            marker_prefix: "#[",
            methods: &["def "],
            inline_bodies: false,
            closing: "end",
        },
    ];

    pub(crate) fn of(ruby: bool) -> Self {
        Self::SYNTAXES[usize::from(ruby)]
    }
}
