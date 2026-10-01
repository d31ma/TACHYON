use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct ScriptServiceReferences;

impl ScriptServiceReferences {
    const OPERATOR_CHARACTERS: &'static str = "=?!+-*/%&|^<>";
    const ASSIGNMENTS: [&'static str; 16] = [
        "=", "??=", "||=", "&&=", "+=", "-=", "*=", "/=", "%=", "**=", "&=", "|=", "^=", "<<=",
        ">>=", ">>>=",
    ];
    const REFLECTIVE_WRITES: [&'static str; 4] = [
        "Object.assign(this,",
        "Object.defineProperty(this,",
        "Object.defineProperties(this,",
        "Reflect.set(this,",
    ];

    pub(super) fn read(path: &Path, source: &str, known: &BTreeSet<String>) -> BTreeSet<String> {
        crate::annotation_layout::masked_source(path, source)
            .map(|code| Self::references(&code, known))
            .unwrap_or_else(|_| known.clone())
    }

    fn references(code: &str, known: &BTreeSet<String>) -> BTreeSet<String> {
        let names = Self::identifiers(code, known);
        let compact = Self::compact(code);
        let computed = Self::computed_write(&compact);
        let reflective = Self::reflective_write(&compact) || Self::escaped_owner(code);
        Self::selected(names, known, (computed, reflective))
    }

    fn identifiers(code: &str, known: &BTreeSet<String>) -> BTreeSet<String> {
        code.split(|character: char| {
            !character.is_alphanumeric() && character != '_' && character != '$'
        })
        .filter(|word| known.contains(*word))
        .map(str::to_owned)
        .collect()
    }

    fn compact(code: &str) -> String {
        code.chars()
            .filter(|character| !character.is_whitespace())
            .collect()
    }

    fn computed_write(code: &str) -> bool {
        code.split("this[")
            .skip(1)
            .filter_map(|part| part.split_once(']'))
            .any(|(_, tail)| Self::is_assignment(tail))
    }

    fn is_assignment(tail: &str) -> bool {
        let operator: String = tail
            .chars()
            .take_while(|character| Self::OPERATOR_CHARACTERS.contains(*character))
            .collect();
        Self::ASSIGNMENTS.contains(&operator.as_str())
    }

    fn reflective_write(code: &str) -> bool {
        Self::REFLECTIVE_WRITES
            .iter()
            .any(|prefix| code.contains(prefix))
    }

    fn escaped_owner(code: &str) -> bool {
        code.match_indices("this").any(|(offset, _)| {
            let before = code[..offset].chars().next_back();
            let after = code[offset + 4..].chars().next();
            let token = !before.is_some_and(Self::identifier_character)
                && !after.is_some_and(Self::identifier_character);
            token && !code[offset + 4..].trim_start().starts_with(['.', '['])
        })
    }

    fn identifier_character(character: char) -> bool {
        character.is_alphanumeric() || matches!(character, '_' | '$')
    }

    fn selected(
        names: BTreeSet<String>,
        known: &BTreeSet<String>,
        writes: (bool, bool),
    ) -> BTreeSet<String> {
        match writes {
            (false, false) => names,
            _ => known.clone(),
        }
    }
}
