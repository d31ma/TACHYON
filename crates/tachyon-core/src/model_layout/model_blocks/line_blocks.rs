use super::super::piece::Piece;
use super::model_block::ModelBlock;

pub(super) struct LineBlocks<'a> {
    code: &'a str,
    lines: Vec<Piece<'a>>,
}

impl<'a> LineBlocks<'a> {
    const CLOSING_STARTS: [&'static str; 3] = [")", "]", "}"];
    const RUBY_END: &'static str = "end";
    const CLASS: &'static str = "class";
    const RUBY_RECORD_BUILDERS: [&'static str; 2] = ["Data.define", "Struct.new"];

    pub(super) fn python(code: &'a str) -> Vec<ModelBlock> {
        let blocks = Self::new(code);
        blocks.read(Self::python_name)
    }

    pub(super) fn fsharp(code: &'a str) -> Vec<ModelBlock> {
        Self::new(code).read(Self::fsharp_name)
    }

    fn fsharp_name(line: Piece<'_>) -> Option<&str> {
        Self::named(line.after_word("type")?.first_word())
    }

    pub(super) fn ruby(code: &'a str) -> Vec<ModelBlock> {
        let blocks = Self::new(code);
        blocks.read(Self::ruby_name)
    }

    fn new(code: &'a str) -> Self {
        let lines = Self::lines(code)
            .into_iter()
            .filter(|line| Self::starts_top_level(*line))
            .collect();
        Self { code, lines }
    }

    fn lines(code: &'a str) -> Vec<Piece<'a>> {
        code.split_inclusive('\n')
            .scan(0, |offset, line| {
                let at = *offset;
                *offset += line.len();
                Some(Piece::new(at, line.trim_end()))
            })
            .collect()
    }

    fn starts_top_level(line: Piece<'_>) -> bool {
        let checks = [
            line.text.trim().is_empty(),
            line.text.starts_with(char::is_whitespace),
            Self::CLOSING_STARTS
                .iter()
                .any(|start| line.starts_with(start)),
            line.first_word() == Self::RUBY_END,
        ];
        !checks.contains(&true)
    }

    fn read(&self, name_of: fn(Piece<'_>) -> Option<&str>) -> Vec<ModelBlock> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| self.block(index, name_of(*line)))
            .collect()
    }

    fn block(&self, index: usize, name: Option<&str>) -> Option<ModelBlock> {
        let name = name?;
        let start = self.lines.get(index)?.at;
        let next = self
            .lines
            .get(index + 1)
            .map_or(self.code.len(), |line| line.at);
        let end = start + self.code[start..next].trim_end().len();
        Some(ModelBlock::new(name, start, end))
    }

    fn python_name(line: Piece<'_>) -> Option<&str> {
        let rest = line.after_word(Self::CLASS)?;
        Self::named(rest.first_word())
    }

    fn ruby_name(line: Piece<'_>) -> Option<&str> {
        let class = line.after_word(Self::CLASS);
        class.map_or_else(
            || Self::ruby_record_name(line),
            |rest| Self::ruby_constant(rest),
        )
    }

    fn ruby_record_name(line: Piece<'_>) -> Option<&str> {
        let (name, value) = line.split_first_top(b'=');
        let builds_record = Self::RUBY_RECORD_BUILDERS
            .iter()
            .any(|builder| value.starts_with(builder));
        builds_record.then_some(name.text).and_then(Self::constant)
    }

    fn ruby_constant(rest: Piece<'_>) -> Option<&str> {
        let path = rest.path();
        Self::constant(Piece::last_segment(path))
    }

    fn constant(name: &str) -> Option<&str> {
        let starts_upper = name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_uppercase());
        let plain = name.bytes().all(Piece::is_name_byte);
        (![starts_upper, plain].contains(&false)).then_some(name)
    }

    fn named(name: &str) -> Option<&str> {
        Some(name).filter(|name| !name.is_empty())
    }
}
