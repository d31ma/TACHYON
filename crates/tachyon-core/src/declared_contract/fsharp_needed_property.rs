pub(super) struct FsharpNeededProperty<'a> {
    indent: &'a str,
    name: &'a str,
    type_name: &'a str,
}

impl<'a> FsharpNeededProperty<'a> {
    const MARKER: &'static str = "[<Needs>]";
    const MEMBER: &'static str = "member val ";
    const HELPER_SUFFIX: &'static str = "Helper";
    const FILLED: &'static str =
        "{indent}member val private {name} : {type} = new {type}() with get, set\n";

    pub(super) fn marker(lines: &[&'a str], at: usize, contracts: &[String]) -> bool {
        let marked = lines[at].trim() == Self::MARKER;
        let property = lines
            .get(at + 1)
            .and_then(|line| Self::read(line))
            .is_some_and(|property| property.helper(contracts));
        [marked, property] == [true, true]
    }

    pub(super) fn needed(lines: &[&'a str], at: usize, contracts: &[String]) -> Option<Self> {
        let marked = at
            .checked_sub(1)
            .is_some_and(|before| lines[before].trim() == Self::MARKER);
        marked.then_some(())?;
        Self::read(lines[at]).filter(|property| property.helper(contracts))
    }

    pub(super) fn filled(&self) -> String {
        Self::FILLED
            .replace("{indent}", self.indent)
            .replace("{name}", self.name)
            .replace("{type}", self.type_name)
    }

    fn read(line: &'a str) -> Option<Self> {
        let indent = &line[..line.len() - line.trim_start().len()];
        let declared = line.trim().strip_prefix(Self::MEMBER)?;
        let (name, rest) = declared.split_once(':')?;
        let (type_name, _) = rest.split_once('=')?;
        let property = Self {
            indent,
            name: name.trim(),
            type_name: type_name.trim(),
        };
        [Self::word(property.name), Self::word(property.type_name)]
            .iter()
            .all(|word| *word)
            .then_some(property)
    }

    fn word(text: &str) -> bool {
        let letters = text
            .chars()
            .filter(|letter| Self::word_letter(*letter))
            .count();
        [text.is_empty(), letters == text.len()] == [false, true]
    }

    fn word_letter(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    fn helper(&self, contracts: &[String]) -> bool {
        let named = contracts.iter().any(|contract| contract == self.type_name);
        [named, self.type_name.ends_with(Self::HELPER_SUFFIX)] == [true, true]
    }
}
