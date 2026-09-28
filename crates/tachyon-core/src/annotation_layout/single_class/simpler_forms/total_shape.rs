use super::code_language::CodeLanguage;

#[derive(Clone, Copy, Debug)]
pub(super) struct TotalShape {
    pub(super) language: CodeLanguage,
    pub(super) start: &'static str,
    pub(super) header: &'static str,
    pub(super) sum_body: &'static str,
    pub(super) join_body: &'static str,
    pub(super) sum_output: &'static str,
    pub(super) join_output: &'static str,
}

impl TotalShape {
    pub(super) const SHAPES: [Self; 6] = [
        Self::new(
            CodeLanguage::Kotlin,
            ["var @T = #Z", "for ( @V in %XS ) {"],
            ["@T += @V", "var {T} = {XS}.sum()"],
            ["@T += @V", "var {T} = {XS}.joinToString(\"\")"],
        ),
        Self::new(
            CodeLanguage::Php,
            ["$ @T = #Z ;", "foreach ( $ %XS as $ @V ) {"],
            ["$ @T += $ @V ;", "${T} = array_sum(${XS});"],
            ["$ @T .= $ @V ;", "${T} = implode('', ${XS});"],
        ),
        Self::new(
            CodeLanguage::Python,
            ["@T = #Z", "for @V in %XS :"],
            ["", ""],
            ["@T += @V", "{T} = \"\".join({XS})"],
        ),
        Self::new(
            CodeLanguage::CSharp,
            ["@D @T = #Z ;", "foreach ( @E @V in %XS ) {"],
            ["", ""],
            ["@T += @V ;", "{D} {T} = string.Concat({XS});"],
        ),
        Self::new(
            CodeLanguage::Dart,
            ["@D @T = #Z ;", "for ( @E @V in %XS ) {"],
            ["", ""],
            ["@T += @V ;", "{D} {T} = {XS}.join();"],
        ),
        Self::new(
            CodeLanguage::Swift,
            ["var @T = #Z", "for @V in %XS {"],
            ["", ""],
            ["@T += @V", "var {T} = {XS}.joined()"],
        ),
    ];

    const fn new(
        language: CodeLanguage,
        lines: [&'static str; 2],
        sum: [&'static str; 2],
        join: [&'static str; 2],
    ) -> Self {
        Self {
            language,
            start: lines[0],
            header: lines[1],
            sum_body: sum[0],
            join_body: join[0],
            sum_output: sum[1],
            join_output: join[1],
        }
    }

    pub(super) fn body(&self, joins: bool) -> &'static str {
        [self.sum_body, self.join_body][usize::from(joins)]
    }

    pub(super) fn output(&self, joins: bool) -> &'static str {
        [self.sum_output, self.join_output][usize::from(joins)]
    }
}
