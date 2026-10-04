use super::MigrationFinding;
use super::built_role::BuiltRole;
use std::path::Path;

pub(super) struct BuiltConverter;

impl BuiltConverter {
    const FEATURE: &'static str = "converter.built_by_hand";
    const SUFFIX: &'static str = "Converter";
    const CONSTRUCT_CALL: &'static str = "construct(";
    const CONSTRUCTORS: [&'static str; 3] = ["(", "::new(", ".new("];
    const EMPTY_CALL: char = ')';
    const GO_LITERAL: char = '{';
    const CONTEXT_FIELD: &'static str = "behaviors";
    const DECLARATION_WORD: &'static str = "class";
    const EXTENSIONS: [&'static str; 14] = [
        "js", "ts", "py", "php", "java", "groovy", "kt", "cs", "fs", "rs", "swift", "dart", "go",
        "rb",
    ];
    const DETAIL: &'static str = "A Converter needs no behavior context: Tachyon builds it, and it has no constructor argument.";
    const ROLE: BuiltRole = BuiltRole {
        feature: Self::FEATURE,
        suffix: Self::SUFFIX,
        detail: Self::DETAIL,
        action: Self::ACTION,
    };
    const ACTION: &'static str = "Remove the code that builds the converter with a context, such as new ProductConverter(context) or Behaviors.construct(ProductConverter, ...), and ask for it with Needs: a Needs field, an F# constructor parameter, or Needs above a Ruby attr_reader.";

    pub(super) fn read_finding(root: &Path, relative: &str) -> Option<MigrationFinding> {
        Self::scans(relative).then_some(())?;
        Self::finding(relative, &BuiltRole::source(root, relative)?)
    }

    fn scans(relative: &str) -> bool {
        Path::new(relative)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| Self::EXTENSIONS.contains(&extension))
    }

    pub(super) fn finding(relative: &str, source: &str) -> Option<MigrationFinding> {
        Self::scans(relative).then_some(())?;
        Self::ROLE.finding(relative, source, Self::builds_with_context)
    }

    fn builds_with_context(source: &str, at: usize) -> bool {
        let before = &source[..at];
        let name_start = BuiltRole::name_start(before);
        let after = &source[at + Self::SUFFIX.len()..];
        let type_name =
            source[name_start..].starts_with(|character: char| character.is_ascii_uppercase());
        let named = [
            name_start < at,
            type_name,
            !after.starts_with(BuiltRole::is_name_character),
        ] == [true, true, true];
        let before_name = before[..name_start].trim_end();
        let constructed = before_name.ends_with(Self::CONSTRUCT_CALL);
        let declared = before_name.ends_with(Self::DECLARATION_WORD);
        let called = [Self::called_with_arguments(after), declared] == [true, false];
        let built = [constructed, called, Self::go_literal_with_context(after)].contains(&true);
        [named, built] == [true, true]
    }

    fn called_with_arguments(after: &str) -> bool {
        Self::CONSTRUCTORS
            .iter()
            .find_map(|opening| after.strip_prefix(opening))
            .is_some_and(|inside| !inside.trim_start().starts_with(Self::EMPTY_CALL))
    }

    fn go_literal_with_context(after: &str) -> bool {
        after
            .strip_prefix(Self::GO_LITERAL)
            .is_some_and(|inside| inside.trim_start().starts_with(Self::CONTEXT_FIELD))
    }
}
