use super::dart_top_level::DartTopLevel;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

#[derive(Default)]
pub(super) struct DartPage {
    directives: BTreeSet<String>,
    declarations: Vec<String>,
    owners: BTreeMap<String, (String, String)>,
}

impl DartPage {
    pub(super) const LIBRARY: &str = "services.dart";
    const DIRECTIVES: [&str; 4] = ["import ", "export ", "library ", "part "];
    const TYPE_KEYWORDS: [&str; 5] = ["class", "enum", "mixin", "typedef", "extension"];
    const MODIFIERS: [&str; 10] = [
        "abstract",
        "base",
        "final",
        "interface",
        "sealed",
        "external",
        "static",
        "const",
        "late",
        "var",
    ];
    const HOSTS: [(&str, &str); 2] = [
        (
            "final class BehaviorStorage {",
            "  page.BehaviorStorage.host = tachyon.tachyonStorageHost;\n",
        ),
        (
            "abstract final class BehaviorBrowserHttp {",
            "  page.BehaviorBrowserHttp.host = (request, signal) => tachyon.tachyonHttpHost(request, signal.watch);\n",
        ),
    ];

    pub(super) fn add(&mut self, portable: &str, source: &str) -> Result<(), String> {
        let code = Self::code(source)?;
        let (directives, declarations): (Vec<_>, Vec<_>) = DartTopLevel::ranges(&code)
            .into_iter()
            .filter(|range| !code[range.clone()].trim().is_empty())
            .partition(|range| Self::is_directive(&code[range.clone()]));
        self.directives.extend(
            directives
                .into_iter()
                .map(|range| Self::text(source, &code, range).to_owned()),
        );
        declarations.into_iter().try_for_each(|range| {
            let name = Self::name(code[range.clone()].trim());
            self.add_declaration(portable, &name, Self::text(source, &code, range))
        })
    }

    pub(super) fn source(&self) -> String {
        let directives: Vec<&str> = self.directives.iter().map(String::as_str).collect();
        format!(
            "{}\n\n{}\n",
            directives.join("\n"),
            self.declarations.join("\n\n")
        )
    }

    pub(super) fn hosts(library: &str) -> String {
        Self::HOSTS
            .iter()
            .filter(|(marker, _)| library.contains(marker))
            .map(|(_, line)| *line)
            .collect()
    }

    fn code(source: &str) -> Result<String, String> {
        let mask = crate::lexical::code_mask("dart", source);
        mask.error.map_or(Ok(()), |error| Err(error.to_owned()))?;
        String::from_utf8(super::masked_code(source, &mask.code))
            .map_err(|_| "the service source is not UTF-8".to_owned())
    }

    fn is_directive(code: &str) -> bool {
        let code = code.trim_start();
        Self::DIRECTIVES.iter().any(|word| code.starts_with(word))
    }

    fn text<'a>(source: &'a str, code: &str, range: Range<usize>) -> &'a str {
        let lead = code[range.clone()].len() - code[range.clone()].trim_start().len();
        source[range.start + lead..range.end].trim_end()
    }

    fn add_declaration(&mut self, portable: &str, name: &str, text: &str) -> Result<(), String> {
        let key = [name, text][usize::from(name.is_empty())];
        match self.owners.get(key) {
            None => {
                self.owners
                    .insert(key.to_owned(), (portable.to_owned(), text.to_owned()));
                self.declarations.push(text.to_owned());
                Ok(())
            }
            Some((_, written)) if written == text => Ok(()),
            Some((owner, _)) => Err(format!(
                "'{name}' is also declared, differently, by {owner}; every Dart browser service \
                 of the site is built into one program, so give one of them another name"
            )),
        }
    }

    fn name(header: &str) -> String {
        let header = super::without_metadata(header);
        let words: Vec<&str> = header.split_whitespace().collect();
        words
            .iter()
            .position(|word| Self::TYPE_KEYWORDS.contains(word))
            .map_or_else(
                || Self::value_name(header),
                |at| Self::type_name(&words[at + 1..]),
            )
    }

    fn type_name(words: &[&str]) -> String {
        let named = words
            .iter()
            .find(|word| **word != "type")
            .copied()
            .filter(|word| *word != "on")
            .unwrap_or_default();
        Self::identifier(named)
    }

    fn value_name(header: &str) -> String {
        let end = header.find(['(', '=', ';', '{']).unwrap_or(header.len());
        let named = header[..end]
            .split_whitespace()
            .rfind(|word| !Self::MODIFIERS.contains(word))
            .unwrap_or_default();
        Self::identifier(named)
    }

    fn identifier(word: &str) -> String {
        word.chars()
            .take_while(|character| super::identifier_character(*character))
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::DartPage;

    const SETUP: &str = "@Setup\nclass PageSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) => BehaviorAdapters();\n}\n";

    #[test]
    fn shared_declarations_and_imports_appear_once() {
        let mut page = DartPage::default();
        let reader = format!(
            "import 'dart:math';\nimport 'models/State.dart';\n{SETUP}\n@Service\nclass ReaderService {{\n  int count = 0;\n}}\n"
        );
        let writer = format!(
            "import 'dart:math';\n{SETUP}\n@Service\nclass WriterService {{\n  final values = {{'a': 1}};\n}}\nfinal table = {{'b': 2}};\n"
        );
        page.add("client/services/ReaderService.dart", &reader)
            .expect("reader");
        page.add("client/services/WriterService.dart", &writer)
            .expect("writer");
        let source = page.source();
        assert_eq!(source.matches("class PageSetup").count(), 1, "{source}");
        assert_eq!(source.matches("import 'dart:math';").count(), 1, "{source}");
        assert!(source.contains("import 'models/State.dart';"), "{source}");
        assert!(source.contains("class ReaderService"), "{source}");
        assert!(source.contains("class WriterService"), "{source}");
        assert!(source.contains("final table = {'b': 2};"), "{source}");
    }

    #[test]
    fn one_name_written_two_ways_is_refused() {
        let mut page = DartPage::default();
        page.add(
            "client/services/ReaderService.dart",
            "class Helper {\n  int value = 1;\n}\n",
        )
        .expect("reader");
        let error = page
            .add(
                "client/services/WriterService.dart",
                "// a comment\nclass Helper {\n  int value = 2;\n}\n",
            )
            .expect_err("conflict");
        assert!(
            error.contains(
                "'Helper' is also declared, differently, by client/services/ReaderService.dart"
            ),
            "{error}"
        );
    }

    #[test]
    fn names_come_from_types_functions_and_values() {
        for (header, name) in [
            (
                "@Entity({\"a\":1})\nabstract interface class Store<T>",
                "Store",
            ),
            ("extension type Wrapped(int value)", "Wrapped"),
            ("Future<void> helper()", "helper"),
            ("final table = ", "table"),
            ("int get size", "size"),
            ("typedef Reader = ", "Reader"),
            ("extension on String", ""),
        ] {
            assert_eq!(DartPage::name(header), name, "{header}");
        }
    }
}
