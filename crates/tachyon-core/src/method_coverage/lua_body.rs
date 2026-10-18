use super::{MethodHeader, PublicMethod, SourceEdit};

pub(crate) struct LuaBody;

impl LuaBody {
    const KEYWORD: &'static str = "function ";
    const RECEIVERS: [char; 2] = [':', '.'];
    const PARAMETERS: char = '(';
    const PARAMETERS_END: char = ')';
    const END: &'static str = "end";
    const PRIVATE: char = '_';

    pub(crate) fn methods(masked: &str) -> Vec<PublicMethod> {
        masked
            .lines()
            .enumerate()
            .filter_map(|(line, text)| {
                Self::public_name(text).map(|name| PublicMethod::new(name, line))
            })
            .filter(|method| !Self::is_empty(masked, method.line))
            .collect()
    }

    fn is_empty(masked: &str, line: usize) -> bool {
        let start: usize = masked.split_inclusive('\n').take(line).map(str::len).sum();
        let rest = masked.get(start..).unwrap_or_default();
        let body = rest
            .find(Self::PARAMETERS_END)
            .map_or("", |close| &rest[close + 1..]);
        body.trim_start()
            .strip_prefix(Self::END)
            .is_some_and(|after| !after.starts_with(Self::is_name_character))
    }

    fn is_name_character(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    pub(crate) fn edits(header: &MethodHeader<'_>) -> Vec<SourceEdit> {
        header
            .parameters_end()
            .map(|end| SourceEdit::insert(end, format!(" {}", header.hook.trim())))
            .into_iter()
            .collect()
    }

    fn public_name(line: &str) -> Option<String> {
        Self::name(line).filter(|name| !name.starts_with(Self::PRIVATE))
    }

    fn name(line: &str) -> Option<String> {
        let (_, method) = line
            .strip_prefix(Self::KEYWORD)?
            .split_once(Self::RECEIVERS)?;
        let (name, _) = method.split_once(Self::PARAMETERS)?;
        Some(name.trim().to_owned())
    }
}
