use super::{LuaBody, ShadowLines, ShadowWriter};

#[derive(Debug, Clone)]
pub(crate) struct LuaShadow {
    lines: ShadowLines,
    text: String,
    problems: Vec<(usize, &'static str)>,
}

impl LuaShadow {
    pub(crate) const SECOND_TYPE: &'static str =
        "A Lua class file holds one class table. Move the second `Name = {}` table to its own file";
    pub(crate) const FOREIGN_METHOD: &'static str = "A method belongs to the class table of its file: write `function Name:method(...)` with the file's class name";
    pub(crate) const STATEMENT: &'static str = "A Lua class file holds only its class table, its constants and its methods. Tachyon cannot read this line: write `Name = {}`, `Name.CONSTANT = value` or `function Name:method(...)`, and keep other code inside methods";
    pub(crate) const ONE_LINE: &'static str = "Put the body of a Lua method on its own lines, and close the method with `end` at the start of a line";
    pub(crate) const LOCAL_TABLE: &'static str = "Tachyon joins the Lua class files of one realm, so a class table is global: write `Name = {}`, not `local Name = {}`";
    pub(crate) const RETURNED_TABLE: &'static str = "A Lua class file does not return its class table: Tachyon joins the class files of one realm, so remove `return Name`";
    pub(crate) const REQUIRED_FILE: &'static str = "A Lua class file does not call `require` for another class: Tachyon joins the class files of one realm, so use the class table by its name";
    pub(crate) const TABLE_FIELD: &'static str =
        "Each field of a class table sits on its own line as `name = value,`";

    pub(crate) fn of(source: &str) -> Self {
        Self::written(source, false, false)
    }

    pub(crate) fn joined(source: &str) -> Self {
        Self::written(source, true, false)
    }

    pub(crate) fn yon(source: &str) -> Self {
        Self::written(source, true, true)
    }

    fn written(source: &str, joined: bool, yon: bool) -> Self {
        let sanitized = LuaBody::sanitized(source);
        let (lines, problems) = ShadowWriter::write(source, &sanitized, joined, yon);
        Self {
            text: lines.text(),
            lines,
            problems,
        }
    }

    const STATEMENT_ADVICE: [(&'static str, &'static str); 3] = [
        ("local ", Self::LOCAL_TABLE),
        ("return ", Self::RETURNED_TABLE),
        ("require", Self::REQUIRED_FILE),
    ];

    pub(crate) fn statement_problem(line: &str) -> &'static str {
        Self::advice_for(line.trim_start())
    }

    fn advice_for(text: &str) -> &'static str {
        Self::STATEMENT_ADVICE
            .iter()
            .find(|(start, _)| text.starts_with(start))
            .map_or(Self::STATEMENT, |(_, advice)| advice)
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn source_line(&self, shadow_line: usize) -> Option<usize> {
        self.lines.source_line(shadow_line)
    }

    pub(crate) fn first_problem(&self) -> Option<(usize, &'static str)> {
        self.problems.first().copied()
    }
}
