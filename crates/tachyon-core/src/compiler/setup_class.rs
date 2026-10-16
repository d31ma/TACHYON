use super::module_code::ModuleCode;
use std::ops::Range;

pub(super) struct SetupClass {
    pub(super) name: String,
    pub(super) span: Range<usize>,
}

impl SetupClass {
    const KEYWORD: &'static str = "class ";
    const STATEMENT_ENDS: &'static str = ";}";
    const MODULE_ACCESS: [&'static str; 2] = ["import", "eval"];

    pub(super) fn find(module: &ModuleCode, name: &str) -> Option<Self> {
        let start = Self::declaration_start(&module.code, name)?;
        let open = start + module.code[start..].find('{')?;
        let close = module.matching_brace(open)?;
        Some(Self {
            name: name.to_owned(),
            span: start..close + 1,
        })
    }

    pub(super) fn shareable(module: &ModuleCode, name: &str) -> Option<Self> {
        Self::find(module, name).filter(|class| class.stands_alone(module))
    }

    pub(super) fn text<'a>(&self, module: &'a ModuleCode) -> &'a str {
        &module.text[self.span.clone()]
    }

    fn declaration_start(code: &str, name: &str) -> Option<usize> {
        let declaration = format!("{}{name}", Self::KEYWORD);
        code.match_indices(&declaration)
            .map(|(at, _)| at)
            .find(|at| Self::starts_statement(code, *at, declaration.len()))
    }

    fn starts_statement(code: &str, at: usize, length: usize) -> bool {
        let before = code[..at].trim_end().chars().last();
        let after = code[at + length..].chars().next().unwrap_or(';');
        let begins = before.is_none_or(|character| Self::STATEMENT_ENDS.contains(character));
        let ends = [after == '{', after.is_whitespace()].contains(&true);
        [begins, ends] == [true, true]
    }

    fn stands_alone(&self, module: &ModuleCode) -> bool {
        let used = ModuleCode::words(&module.code[self.span.clone()]);
        let mut names = module.top_level_names(self.span.clone());
        names.remove(&self.name);
        let reaches_module = Self::MODULE_ACCESS.iter().any(|word| used.contains(*word));
        let uses_module_name = !used.is_disjoint(&names);
        [reaches_module, uses_module_name] == [false, false]
    }
}
