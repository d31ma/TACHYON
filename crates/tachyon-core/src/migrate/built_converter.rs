use super::{MigrationFinding, MigrationStatus};
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
    const ACTION: &'static str = "Remove the code that builds the converter with a context, such as new ProductConverter(context) or Behaviors.construct(ProductConverter, ...), and ask for it with Needs: a Needs field, an F# constructor parameter, or Needs above a Ruby attr_reader.";

    pub(super) fn scans(relative: &str) -> bool {
        Path::new(relative)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| Self::EXTENSIONS.contains(&extension))
    }

    pub(super) fn finding(relative: &str, source: &str) -> Option<MigrationFinding> {
        Self::scans(relative).then_some(())?;
        let extension = Path::new(relative).extension()?.to_str()?;
        let mask = crate::lexical::code_mask(extension, source);
        let built = source
            .match_indices(Self::SUFFIX)
            .filter(|(at, _)| mask.code.get(*at) == Some(&true))
            .any(|(at, _)| Self::builds_with_context(source, at));
        built.then(|| {
            MigrationFinding::new(
                relative,
                Self::FEATURE,
                MigrationStatus::Changed,
                Self::DETAIL,
                Some(Self::ACTION),
            )
        })
    }

    fn builds_with_context(source: &str, at: usize) -> bool {
        let before = &source[..at];
        let name_start = before
            .rfind(|character: char| !Self::is_name_character(character))
            .map_or(0, |found| found + 1);
        let after = &source[at + Self::SUFFIX.len()..];
        let type_name =
            source[name_start..].starts_with(|character: char| character.is_ascii_uppercase());
        let named = [
            name_start < at,
            type_name,
            !after.starts_with(Self::is_name_character),
        ] == [true, true, true];
        let before_name = before[..name_start].trim_end();
        let constructed = before_name.ends_with(Self::CONSTRUCT_CALL);
        let declared = before_name.ends_with(Self::DECLARATION_WORD);
        let called = [Self::called_with_arguments(after), declared] == [true, false];
        named && [constructed, called, Self::go_literal_with_context(after)].contains(&true)
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

    fn is_name_character(character: char) -> bool {
        character.is_ascii_alphanumeric() || character == '_'
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltConverter;

    #[test]
    fn a_converter_built_with_a_context_is_reported_in_every_form() {
        let built = [
            (
                "A.java",
                "ProductConverter mapper = new ProductConverter(context);",
            ),
            ("A.kt", "val mapper = ProductConverter(context)"),
            (
                "A.rs",
                "let mapper = ProductConverter::new(context.clone());",
            ),
            (
                "A.js",
                "const mapper = Behaviors.construct(ProductConverter, { behaviors: context });",
            ),
            (
                "A.php",
                "$mapper = Behaviors::construct(ProductConverter::class, ['behaviors' => $context]);",
            ),
            ("A.go", "mapper := &ProductConverter{behaviors: context}"),
            ("A.rb", "mapper = ProductConverter.new(context)"),
        ];
        for (path, source) in built {
            let finding = BuiltConverter::finding(path, source);
            assert!(finding.is_some(), "{path}: {source}");
        }
    }

    #[test]
    fn an_injected_or_declared_converter_is_not_reported() {
        let kept = [
            ("A.java", "@Needs\nProductConverter converter;"),
            ("A.py", "class ProductConverter(Protocol):\n    pass\n"),
            (
                "A.fs",
                "type CatalogController(converter: ProductConverter) =",
            ),
            (
                "A.swift",
                "private let converter: ProductConverter = ProductConverter()",
            ),
            ("A.java", "// new ProductConverter(context)"),
            ("A.java", "@Converter\ninterface ProductConverter {}"),
            ("A.kt", "val converter = makeConverter(settings)"),
        ];
        for (path, source) in kept {
            assert!(
                BuiltConverter::finding(path, source).is_none(),
                "{path}: {source}"
            );
        }
    }
}
